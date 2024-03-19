use terminal::terminal_process::TerminalProcess;
use cgmath::Vector2;
use std::error::Error;
use std::io::{Read,Write};

pub struct ConptyProcess {
    process: conpty::Process,
}

impl ConptyProcess {
    pub fn new(process: conpty::Process) -> Self {
        Self {
            process
        }
    }
}

impl TerminalProcess for ConptyProcess {
    fn terminate(&mut self) -> Result<(), Box<dyn Error>> {
        self.process.exit(0)?;
        Ok(())
    }

    fn get_write_pipe(&mut self) -> Box<dyn Write + Send> {
        let write_pipe = self.process.input().unwrap();
        Box::new(write_pipe)
    }

    fn get_read_pipe(&mut self) -> Box<dyn Read + Send> {
        let read_pipe = self.process.output().unwrap();
        Box::new(read_pipe)
    }

    fn set_size(&mut self, size: Vector2<usize>) -> Result<(), Box<dyn Error>> {
        self.process.resize(size.x as i16, size.y as i16)?;
        Ok(())
    }

    fn is_newline_carriage_return(&self) -> bool {
        // conpty converts \n to \r\n
        false
    }
}

impl Drop for ConptyProcess {
    fn drop(&mut self) {
        if let Err(err) = self.terminate() {
            log::error!("Failed to close conpty process: {:?}", err);
        }
    }
}
