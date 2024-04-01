use crate::process::TerminalProcess;
use terminal::TerminalIOControl;
use std::io::{Read, Write};
use std::process::Child;

pub struct RawProcess {
    process: Child,
}

impl RawProcess {
    pub fn new(process: Child) -> Self {
        Self {
            process
        }
    }
}

impl TerminalProcess for RawProcess {
    fn terminate(&mut self) -> anyhow::Result<()> {
        self.process.kill()?;
        Ok(())
    }

    fn get_write_pipe(&mut self) -> anyhow::Result<Box<dyn Write + Send>> {
        if cfg!(windows) || cfg!(unix) {
            use std::process::ChildStdin;
            let stdin = self.process.stdin.as_ref().expect("stdin cannot be taken from child");
            #[cfg(windows)] {
                use std::os::windows::io::AsHandle;
                let cloned_handle = stdin.as_handle().try_clone_to_owned()?;
                Ok(Box::new(ChildStdin::from(cloned_handle)))
            }
            #[cfg(unix)] {
                use std::os::fd::AsFd;
                let cloned_fd = stdin.as_fd().try_clone_to_owned()?;
                Ok(Box::new(ChildStdin::from(cloned_fd)))
            }
        } else {
            match self.process.stdin.take() {
                Some(write_pipe) => Ok(Box::new(write_pipe)),
                None => Err(anyhow::Error::msg("stdin taken from child already")),
            }
        }
    }

    fn get_read_pipe(&mut self) -> anyhow::Result<Box<dyn Read + Send>> {
        if cfg!(windows) || cfg!(unix) {
            use std::process::ChildStdout;
            let stdout = self.process.stdout.as_ref().expect("stdout cannot be taken from child");
            #[cfg(windows)] {
                use std::os::windows::io::AsHandle;
                let cloned_handle = stdout.as_handle().try_clone_to_owned()?;
                Ok(Box::new(ChildStdout::from(cloned_handle)))
            }
            #[cfg(unix)] {
                use std::os::fd::AsFd;
                let cloned_fd = stdout.as_fd().try_clone_to_owned()?;
                Ok(Box::new(ChildStdout::from(cloned_fd)))
            }
        } else {
            match self.process.stdout.take() {
                Some(read_pipe) => Ok(Box::new(read_pipe)),
                None => Err(anyhow::Error::msg("stdout taken from child already")),
            }
        }
    }

    fn on_ioctl(&mut self, _ev: TerminalIOControl) -> anyhow::Result<()> {
        Ok(())
    }

    fn is_newline_carriage_return(&self) -> bool {
        true
    }
}

impl Drop for RawProcess {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}
