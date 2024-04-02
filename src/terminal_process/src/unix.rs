use crate::process::TerminalProcess;
use terminal::TerminalIOControl;
use std::io::{Read, Write};
use unix_pty::process::PtyProcess as Process;
use cgmath::Vector2;

pub struct UnixPtyProcess {
    process: Process,    
}

impl UnixPtyProcess {
    pub fn new(process: Process) -> Self {
        Self { 
            process 
        } 
    }
}

impl TerminalProcess for UnixPtyProcess {
    fn terminate(&mut self) -> anyhow::Result<()> {
        self.process.kill()?;
        Ok(())
    }

    fn get_write_pipe(&mut self) -> anyhow::Result<Box<dyn Write + Send>> {
        let master_pty = self.process.get_master_pty().try_clone()?;
        Ok(Box::new(master_pty))
    }

    fn get_read_pipe(&mut self) -> anyhow::Result<Box<dyn Read + Send>> {
        let master_pty = self.process.get_master_pty().try_clone()?;
        Ok(Box::new(master_pty))
    }

    fn on_ioctl(&mut self, ev: TerminalIOControl) -> anyhow::Result<()> {
        match ev {
            TerminalIOControl::SetSize(size) => {
                let size = Vector2::new(size.x as u16, size.y as u16);
                self.process.get_master_pty().set_window_size(size)?
            },
        }
        Ok(())
    }

    fn is_newline_carriage_return(&self) -> bool {
        false
    }
}
