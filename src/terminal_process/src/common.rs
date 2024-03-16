use terminal::terminal_process::TerminalProcess;
use cgmath::Vector2;
use std::error::Error;
use std::io::{Read,Write};
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
    fn terminate(&mut self) -> Result<(), Box<dyn Error>> {
        self.process.kill()?;
        Ok(())
    }

    fn get_write_pipe(&mut self) -> Box<dyn Write + Send> {
        let write_pipe = self.process.stdin.take().unwrap();
        Box::new(write_pipe)
    }

    fn get_read_pipe(&mut self) -> Box<dyn Read + Send> {
        let read_pipe = self.process.stdout.take().unwrap();
        Box::new(read_pipe)
    }

    fn set_size(&mut self, _size: Vector2<usize>) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

impl Drop for RawProcess {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}
