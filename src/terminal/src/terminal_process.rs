use std::io::{Write,Read};
use std::error::Error;
use cgmath::Vector2;

pub trait TerminalProcess {
    fn terminate(&mut self) -> Result<(), Box<dyn Error>>;
    fn get_write_pipe(&mut self) -> Box<dyn Write + Send>;
    fn get_read_pipe(&mut self) -> Box<dyn Read + Send>;
    fn set_size(&mut self, size: Vector2<usize>) -> Result<(), Box<dyn Error>>;
    // indicates to terminal whether \n should be treated like \r\n
    fn is_newline_carriage_return(&self) -> bool; 
}
