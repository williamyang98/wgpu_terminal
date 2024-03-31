use windows::Win32::{
    Foundation::{
        CloseHandle, HANDLE,
        DuplicateHandle, DUPLICATE_SAME_ACCESS, 
    },
    System::{
        Threading::GetCurrentProcess,
        Pipes::CreatePipe,
    },
    Storage::FileSystem::{
        ReadFile, WriteFile, FlushFileBuffers,
    },
};
use std::io::{Read,Write};

#[derive(Debug)]
pub struct ReadPipe(pub(crate) HANDLE);

#[derive(Debug)]
pub struct WritePipe(pub(crate) HANDLE);

fn convert_error(error: windows::core::Error) -> std::io::Error {
    let code = error.code();
    std::io::Error::from_raw_os_error(code.0)
}

impl Read for ReadPipe {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let mut total_read: u32 = 0;
        unsafe { 
            ReadFile(
                self.0,
                Some(buf),
                Some(&mut total_read),
                None,
            ) 
        }.map_err(convert_error)?;
        Ok(total_read as usize)
    }
}

impl Write for WritePipe {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        let mut total_write: u32 = 0;
        unsafe { 
            WriteFile(
                self.0,
                Some(buf),
                Some(&mut total_write),
                None,
            ) 
        }.map_err(convert_error)?;
        Ok(total_write as usize)
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        unsafe { FlushFileBuffers(self.0) }.map_err(convert_error)
    }
}

impl Drop for ReadPipe {
    fn drop(&mut self) {
        if let Err(err) = unsafe { CloseHandle(self.0) } {
            log::error!("Failed to close read pipe: {:?}", err);
        }
    }
}

impl Drop for WritePipe {
    fn drop(&mut self) {
        if let Err(err) = unsafe { CloseHandle(self.0) } {
            log::error!("Failed to close write pipe: {:?}", err);
        }
    }
}

impl ReadPipe {
    pub fn try_clone(&self) -> Result<Self, windows::core::Error> {
        let source_process = unsafe { GetCurrentProcess() } ;
        let source_handle = self.0;
        let target_process = source_process;
        let mut target_handle = HANDLE::default();
        let desired_access = 0; // ignore when DUPLICATE_SAME_ACCESS
        let inherit_handle = false;
        let options = DUPLICATE_SAME_ACCESS;
        unsafe {
            DuplicateHandle(
                source_process,
                source_handle,
                target_process,
                &mut target_handle,
                desired_access,
                inherit_handle,
                options,
            )
        }?;
        Ok(Self(target_handle))
    }
}

impl WritePipe {
    pub fn try_clone(&self) -> Result<Self, windows::core::Error> {
        // https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-duplicatehandle
        let source_process = unsafe { GetCurrentProcess() } ;
        let source_handle = self.0;
        let target_process = source_process;
        let mut target_handle = HANDLE::default();
        let desired_access = 0; // ignore when DUPLICATE_SAME_ACCESS
        let inherit_handle = false;
        let options = DUPLICATE_SAME_ACCESS;
        unsafe {
            DuplicateHandle(
                source_process,
                source_handle,
                target_process,
                &mut target_handle,
                desired_access,
                inherit_handle,
                options,
            )
        }?;
        Ok(Self(target_handle))
    }
}

pub fn create_pipe(buffer_size: Option<u32>) -> Result<(ReadPipe, WritePipe), windows::core::Error> {
    // https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-createpipe
    let mut read_pipe = HANDLE::default();
    let mut write_pipe = HANDLE::default();
    let attributes = None;
    let buffer_size = buffer_size.unwrap_or(0);
    unsafe {
        CreatePipe(
            &mut read_pipe,
            &mut write_pipe,
            attributes,
            buffer_size,
        )
    }?;
    let read_pipe = ReadPipe(read_pipe);
    let write_pipe = WritePipe(write_pipe);
    Ok((read_pipe, write_pipe))
}

