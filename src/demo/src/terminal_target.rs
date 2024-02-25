use cgmath::Vector2;
use std::io::Write;

pub trait TerminalTarget {
    fn write_data(&mut self, data: &[u8]) -> anyhow::Result<usize>;
    fn set_size(&mut self, size: Vector2<usize>) -> anyhow::Result<Vector2<usize>>;
}

pub struct ConptyTarget<'a> {
    pub process: &'a mut conpty::Process,
    pub pipe_input: &'a mut conpty::io::PipeWriter,
}

impl<'a> TerminalTarget for ConptyTarget<'a> {
    fn write_data(&mut self, data: &[u8]) -> anyhow::Result<usize> {
        Ok(self.pipe_input.write(data)?)
    }

    fn set_size(&mut self, size: Vector2<usize>) -> anyhow::Result<Vector2<usize>> {
        self.process.resize(size.x as i16, size.y as i16)?;
        Ok(size)
    }
}

impl<T: Write> TerminalTarget for T {
    fn write_data(&mut self, data: &[u8]) -> anyhow::Result<usize> {
        Ok(self.write(data)?)
    }

    fn set_size(&mut self, size: Vector2<usize>) -> anyhow::Result<Vector2<usize>> {
        Ok(size)
    }
}

