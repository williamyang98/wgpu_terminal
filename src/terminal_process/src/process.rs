use terminal::TerminalIOControl;
use std::io::{Read, Write};

pub trait TerminalProcess {
    fn on_ioctl(&mut self, ev: TerminalIOControl) -> anyhow::Result<()>;
    fn get_write_pipe(&mut self) -> anyhow::Result<Box<dyn Write + Send>>;
    fn get_read_pipe(&mut self) -> anyhow::Result<Box<dyn Read + Send>>;
    fn terminate(&mut self) -> anyhow::Result<()>;
    // should \n be treated as \r\n?
    fn is_newline_carriage_return(&self) -> bool;
}
