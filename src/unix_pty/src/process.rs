use nix::{
    errno::Errno,
    pty::{openpty, Winsize},
    sys::signal::{signal, Signal, SigHandler},
    unistd::{setsid, Pid},
};
use cgmath::Vector2;
use thiserror::Error;
use std::{
    process::{Command, Child, Stdio, ExitStatus},
    os::{
        unix::process::CommandExt,
        fd::AsRawFd,
    },
};
use crate::master_pty::MasterPty;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct ProcessBuilder {
    pub size: Vector2<u16>,
}

impl Default for ProcessBuilder {
    fn default() -> Self {
        Self {
            size: Vector2::new(128,32),
        }
    }
}

#[derive(Debug)]
pub struct PtyProcess {
    master_pty: MasterPty,
    child: Child,
}

#[derive(Debug,Error)]
pub enum SpawnError {
    #[error("failed to openpty pipes: {0:?}")]
    FailedOpenPty(Errno),
    #[error("failed to set utf8 to pty: {0:?}")]
    FailedSetUtf8MasterPty(Errno),
    #[error("failed to grant access to slave pty: {0:?}")]
    FailedGrantAccessMasterPty(Errno),
    #[error("failed to unlock slave pty: {0:?}")]
    FailedUnlockMasterPty(Errno),
    #[error("failed to clone slave fd: {0:?}")]
    FailedCloneSlaveFd(std::io::Error),
    #[error("failed to spawn process: {0:?}")]
    FailedSpawnProcess(std::io::Error),
}

#[derive(Debug,Error)]
enum ExecError {
    #[error("failed to create new session and set process group id: {0:?}")]
    FailedNewSession(Errno),
    #[error("failed to set as controlling terminal of process: {0:?}")]
    FailedSetControllingTerminal(Errno),
    #[error("failed to set signal handler")]
    FailedSetSignalHandler(Signal, SigHandler, Errno),
}

impl From<ExecError> for std::io::Error {
    fn from(err: ExecError) -> Self {
        Self::new(std::io::ErrorKind::Other, Box::new(err))
    }
}

impl PtyProcess {
    pub fn spawn(mut command: Command, builder: Option<ProcessBuilder>) -> Result<Self, SpawnError> {
        let builder = builder.unwrap_or_default();
        let window_size = Winsize {
            ws_row: builder.size.y,
            ws_col: builder.size.x,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let res = openpty(Some(&window_size), None).map_err(SpawnError::FailedOpenPty)?;
        let master_pty = MasterPty::from(res.master);
        let slave_fd = res.slave;

        master_pty.set_utf8(true).map_err(SpawnError::FailedSetUtf8MasterPty)?;
        master_pty.grantpt().map_err(SpawnError::FailedGrantAccessMasterPty)?;
        master_pty.unlockpt().map_err(SpawnError::FailedUnlockMasterPty)?;
 
        command.stdin(Stdio::from(slave_fd.try_clone().map_err(SpawnError::FailedCloneSlaveFd)?));
        command.stdout(Stdio::from(slave_fd.try_clone().map_err(SpawnError::FailedCloneSlaveFd)?));
        command.stderr(Stdio::from(slave_fd.try_clone().map_err(SpawnError::FailedCloneSlaveFd)?));
        unsafe {
            command.pre_exec(move || -> Result<(), std::io::Error> {
                let _ = setsid().map_err(ExecError::FailedNewSession)?;
                // https://man7.org/linux/man-pages/man2/ioctl_tty.2.html
                let res = libc::ioctl(slave_fd.as_raw_fd(), libc::TIOCSCTTY as _, 0);
                if res != 0 {
                    if res != -1 {
                        log::warn!("Got unexpected failure code: {}", res);
                    }
                    return Err(ExecError::FailedSetControllingTerminal(Errno::last()).into()); 
                }
                // setup signal handlers
                let set_signal = |sig: Signal, handler: SigHandler| {
                    signal(sig, handler).map_err(|e| ExecError::FailedSetSignalHandler(sig, handler, e))
                };
                set_signal(Signal::SIGCHLD, SigHandler::SigDfl)?;
                set_signal(Signal::SIGHUP, SigHandler::SigDfl)?;
                set_signal(Signal::SIGINT, SigHandler::SigDfl)?;
                set_signal(Signal::SIGQUIT, SigHandler::SigDfl)?;
                set_signal(Signal::SIGTERM, SigHandler::SigDfl)?;
                set_signal(Signal::SIGALRM, SigHandler::SigDfl)?;
                Ok(())
            });
        }

        let child = command.spawn().map_err(SpawnError::FailedSpawnProcess)?;

        Ok(Self {
            master_pty,
            child,
        })
    }

    pub fn get_master_pty(&self) -> &MasterPty {
        &self.master_pty
    }

    pub fn get_pid(&self) -> Pid {
        Pid::from_raw(self.child.id() as i32)
    }
 
    pub fn kill(&mut self) -> Result<(), std::io::Error> {
        self.child.kill()
    }

    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>, std::io::Error> {
        self.child.try_wait()
    }

    pub fn wait(&mut self) -> Result<ExitStatus, std::io::Error> {
        self.child.wait()
    }
}
