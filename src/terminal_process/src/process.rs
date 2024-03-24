use terminal::TerminalIOControl;
use std::io::{Read, Write};
use std::error::Error;

pub trait TerminalProcess {
    fn on_ioctl(&mut self, ev: TerminalIOControl) -> Result<(), Box<dyn Error>>;
    fn get_write_pipe(&mut self) -> Box<dyn Write + Send>;
    fn get_read_pipe(&mut self) -> Box<dyn Read + Send>;
    fn terminate(&mut self) -> Result<(), Box<dyn Error>>;
    // should \n be treated as \r\n?
    fn is_newline_carriage_return(&self) -> bool;
}
