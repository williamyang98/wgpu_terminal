use crate::process::TerminalProcess;
use terminal::TerminalIOControl;
use std::io::{Read, Write};
use conpty::process::{ConptyProcess as Process, Size};

pub struct ConptyProcess {
    process: Process,
}

impl ConptyProcess {
    pub fn new(process: Process) -> Self {
        Self {
            process
        }
    }
}

impl TerminalProcess for ConptyProcess {
    fn terminate(&mut self) -> anyhow::Result<()> {
        self.process.terminate(0)?;
        Ok(())
    }

    fn get_write_pipe(&mut self) -> anyhow::Result<Box<dyn Write + Send>> {
        let write_pipe = self.process.get_write_pipe().try_clone()?;
        Ok(Box::new(write_pipe))
    }

    fn get_read_pipe(&mut self) -> anyhow::Result<Box<dyn Read + Send>> {
        let read_pipe = self.process.get_read_pipe().try_clone()?;
        Ok(Box::new(read_pipe))
    }

    fn on_ioctl(&mut self, ev: TerminalIOControl) -> anyhow::Result<()> {
        match ev {
            TerminalIOControl::SetSize(size) => self.process.set_size(Size::new(size.x as i16, size.y as i16))?,
        }
        Ok(())
    }

    fn is_newline_carriage_return(&self) -> bool {
        // conpty converts \n to \r\n
        false
    }
}
