use crate::pipe::{create_pipe, ReadPipe, WritePipe};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{
            CloseHandle,
            WAIT_OBJECT_0, WAIT_ABANDONED, WAIT_TIMEOUT, WAIT_FAILED,
            GetLastError,
        },
        System::{
            Console::{
                CreatePseudoConsole, ClosePseudoConsole, ResizePseudoConsole, HPCON, COORD,
            },
            Threading::{
                InitializeProcThreadAttributeList, DeleteProcThreadAttributeList, UpdateProcThreadAttribute,
                LPPROC_THREAD_ATTRIBUTE_LIST, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
                CreateProcessW, STARTUPINFOEXW, PROCESS_INFORMATION, PROCESS_CREATION_FLAGS,
                CREATE_UNICODE_ENVIRONMENT, EXTENDED_STARTUPINFO_PRESENT,
                WaitForSingleObject, GetProcessId, GetExitCodeProcess, TerminateProcess,
            },
        },
    },
};
use std::process::Command;
use std::time::Duration;
use std::ffi::{OsString, OsStr};
use std::os::windows::ffi::OsStrExt;
use thiserror::Error;

#[repr(C)]
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Size {
    width: i16,
    height: i16,
}

impl Size {
    pub fn new(width: i16, height: i16) -> Self {
        Self { width, height }
    }
}

impl From<Size> for COORD {
    fn from(size: Size) -> Self {
        Self {
            X: size.width,
            Y: size.height,
        }
    }
}

impl Default for Size {
    fn default() -> Self {
        Self {
            width: 128,
            height: 32,
        }
    }
}

#[derive(Debug)]
struct PseudoConsole(HPCON);

impl PseudoConsole {
    fn new(write_pipe: WritePipe, read_pipe: ReadPipe, size: Size) -> Result<Self, windows::core::Error> {
        // https://learn.microsoft.com/en-us/windows/console/createpseudoconsole
        let flags = 0;
        unsafe { CreatePseudoConsole(size.into(), read_pipe.0, write_pipe.0, flags) }.map(Self)
    }
}

impl Drop for PseudoConsole {
    fn drop(&mut self) {
        unsafe { ClosePseudoConsole(self.0) }
    }
}

struct ThreadAttributeList(Vec<u8>);

impl ThreadAttributeList {
    fn new(console: &PseudoConsole) -> Result<Self, SpawnError> {
        // https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-initializeprocthreadattributelist
        // initial call with nullptr to get size of attribute list
        let attribute_count = 1;
        let flags = 0;
        let mut size: usize = 0;
        let res = unsafe {
            InitializeProcThreadAttributeList(
                LPPROC_THREAD_ATTRIBUTE_LIST(std::ptr::null_mut() as _),
                attribute_count, 
                flags, 
                &mut size,
            )
        };
        if size == 0 {
            return Err(SpawnError::FailedGetThreadAttributeListSize(res.err()));
        }
        // actually initialise
        let mut thread_attribute_list = vec![0u8; size];
        unsafe { 
            InitializeProcThreadAttributeList(
                LPPROC_THREAD_ATTRIBUTE_LIST(thread_attribute_list.as_mut_ptr() as _),
                attribute_count, 
                flags,
                &mut size,
            )
        }.map_err(SpawnError::FailedInitialiseThreadAttributeList)?;
        // https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute
        let flags = 0;
        let attribute = PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE;
        unsafe {
            UpdateProcThreadAttribute(
                LPPROC_THREAD_ATTRIBUTE_LIST(thread_attribute_list.as_mut_ptr() as _),
                flags,
                attribute as usize,
                Some(console.0.0 as _),
                std::mem::size_of::<HPCON>(),
                None,
                None,
            )
        }.map_err(SpawnError::FailedUpdateProcessThreadAttribute)?; 
        Ok(Self(thread_attribute_list))
    }

    fn get_mut_ptr(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        LPPROC_THREAD_ATTRIBUTE_LIST(self.0.as_mut_ptr() as _)
    }
}

impl Drop for ThreadAttributeList {
    fn drop(&mut self) {
        unsafe { DeleteProcThreadAttributeList(self.get_mut_ptr()) };
    }
}

#[derive(Debug)]
pub struct ConptyProcess {
    size: Size,
    read_pipe: ReadPipe,
    write_pipe: WritePipe,
    console: PseudoConsole,
    process_info: PROCESS_INFORMATION,
}

unsafe impl Send for ConptyProcess {}

#[derive(Debug,Clone,PartialEq,Eq,Error)]
pub enum SpawnError {
    #[error("failed to create parent to child pipe: {0:?}")]
    FailedCreateParentToChildPipe(windows::core::Error),
    #[error("failed to create child to parent pipe: {0:?}")]
    FailedCreateChildToParentPipe(windows::core::Error),
    #[error("failed to create psuedo console: {0:?}")]
    FailedCreatePseudoConsole(windows::core::Error),
    #[error("failed to create process: {0:?}")]
    FailedCreateProcess(windows::core::Error),
    #[error("failed to get thread attribute list size: {0:?}")]
    FailedGetThreadAttributeListSize(Option<windows::core::Error>),
    #[error("failed to initialise thread attribute list: {0:?}")]
    FailedInitialiseThreadAttributeList(windows::core::Error),
    #[error("failed to update thread attribute list from pseudo-terminal handle: {0:?}")]
    FailedUpdateProcessThreadAttribute(windows::core::Error),
}

#[derive(Debug,Clone,PartialEq,Eq,Error)]
pub enum WaitError {
    #[error("process mutex was abandoned")]
    AbandonedMutex,
    #[error("process wait timeout elapsed")]
    TimeoutElapsed,
    #[error("process wait failed with: {0:?}")]
    FailedWait(Option<windows::core::Error>),
    #[error("process failed to get exit code: {0:?}")]
    FailedGetExitCode(windows::core::Error),
}

fn osstr_to_wchar(string: &OsStr) -> Vec<u16> {
    string
        .encode_wide()
        .chain(Some(0)) // null terminator
        .collect()
}

#[derive(Debug,Clone,Copy,Default,PartialEq,Eq)]
pub struct ConptyProcessBuilder {
    window_size: Size,
    buffer_size: Option<u32>,
}

impl ConptyProcess {
    pub fn spawn(command: Command, builder: Option<ConptyProcessBuilder>) -> Result<Self, SpawnError> {
        let builder = builder.unwrap_or_default();
        let (read_child, write_parent) = create_pipe(builder.buffer_size)
            .map_err(SpawnError::FailedCreateParentToChildPipe)?;
        let (read_parent, write_child) = create_pipe(builder.buffer_size)
            .map_err(SpawnError::FailedCreateChildToParentPipe)?;
        let console = PseudoConsole::new(write_child, read_child, builder.window_size)
            .map_err(SpawnError::FailedCreatePseudoConsole)?;

        // process startup
        let process_attributes = None; // default
        let thread_attributes = None; // default
        let inherit_handles = false;
        // https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags
        let mut creation_flags = PROCESS_CREATION_FLAGS::default();

        // https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw
        let application_name = std::ptr::null();
        let mut command_line = OsString::default();
        command_line.push(command.get_program());
        for arg in command.get_args() {
            command_line.push(" ");
            command_line.push(arg);
        }
        let mut command_line = osstr_to_wchar(command_line.as_os_str());
        let current_directory = command.get_current_dir().map(|dir| osstr_to_wchar(dir.as_os_str()));
        // environment string encoded as unicode
        let mut environment = Vec::<u16>::new();
        for (key, value) in command.get_envs() {
            let Some(value) = value else {
                continue;
            };
            environment.extend(key.encode_wide());
            environment.extend("=".encode_utf16());
            environment.extend(value.encode_wide());
            environment.push(0);
        }
        // inherit parent process environment if no environment available
        let environment = if environment.is_empty() {
            None
        } else {
            environment.push(0); // terminate environment block
            Some(&environment)
        };
        creation_flags |= CREATE_UNICODE_ENVIRONMENT;

        // startup info (extended with thread attribute list)
        // https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session#preparing-for-creation-of-the-child-process
        // https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-startupinfoexw
        let mut thread_attribute_list = ThreadAttributeList::new(&console)?;
        let mut startup_info = STARTUPINFOEXW::default();
        startup_info.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        startup_info.lpAttributeList = thread_attribute_list.get_mut_ptr();
        creation_flags |= EXTENDED_STARTUPINFO_PRESENT;
        // create process
        let mut process_info = PROCESS_INFORMATION::default();
        unsafe {
            CreateProcessW(
                PCWSTR(application_name),
                PWSTR(command_line.as_mut_ptr()), // caller can modify contents
                process_attributes,
                thread_attributes,
                inherit_handles,
                creation_flags,
                environment.map(|buf| buf.as_ptr() as _),
                PCWSTR(current_directory.as_ref().map_or(std::ptr::null(), |buf| buf.as_ptr())),
                &startup_info.StartupInfo,
                &mut process_info,
            )
        }.map_err(SpawnError::FailedCreateProcess)?;

        Ok(ConptyProcess {
            size: builder.window_size,
            read_pipe: read_parent,
            write_pipe: write_parent,
            console,
            process_info,
        })
    }

    pub fn get_pid(&self) -> u32 {
        unsafe { GetProcessId(self.process_info.hProcess) }
    }

    pub fn wait(&self, timeout: Option<Duration>) -> Result<u32, WaitError> {
        // https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject
        use windows::Win32::System::Threading::INFINITE;
        let millis: u32 = match timeout {
            Some(timeout) => {
                timeout
                    .as_millis()
                    .try_into()
                    .unwrap_or(INFINITE)
            },
            None => INFINITE,
        };
        let res = unsafe { WaitForSingleObject(self.process_info.hProcess, millis) };
        #[allow(clippy::wildcard_in_or_patterns)]
        match res {
            WAIT_OBJECT_0 => {
                let mut exit_code: u32 = 0;
                unsafe { GetExitCodeProcess(self.process_info.hProcess, &mut exit_code) }
                    .map_err(WaitError::FailedGetExitCode)?;
                Ok(exit_code)
            },
            WAIT_ABANDONED => Err(WaitError::AbandonedMutex),
            WAIT_TIMEOUT => Err(WaitError::TimeoutElapsed),
            WAIT_FAILED | _ => {
                let err = unsafe { GetLastError() }.err();
                Err(WaitError::FailedWait(err))
            },
        }
    }

    pub fn terminate(&self, exit_code: u32) -> Result<(), windows::core::Error> {
        unsafe { TerminateProcess(self.process_info.hProcess, exit_code) }
    }

    pub fn is_alive(&self) -> bool {
        unsafe { WaitForSingleObject(self.process_info.hProcess, 0u32) == WAIT_TIMEOUT }
    }

    pub fn set_size(&mut self, size: Size) -> Result<(), windows::core::Error> {
        unsafe { ResizePseudoConsole(self.console.0, size.into()) }?;
        self.size = size;
        Ok(())
    }

    pub fn get_size(&self) -> Size {
        self.size
    }

    pub fn get_write_pipe(&self) -> &WritePipe {
        &self.write_pipe
    }

    pub fn get_read_pipe(&self) -> &ReadPipe {
        &self.read_pipe
    }
}

impl Drop for ConptyProcess {
    fn drop(&mut self) {
        if let Err(err) = unsafe { CloseHandle(self.process_info.hProcess) } {
            log::error!("Failed to close process: {:?}", err);
        }
        if let Err(err) = unsafe { CloseHandle(self.process_info.hThread) } {
            log::error!("Failed to close thread: {:?}", err);
        }
    }
}
