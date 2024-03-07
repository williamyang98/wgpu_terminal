use circular_buffer::{CircularBuffer, get_allocation_granularity};
use crate::primitives::Cell;

#[derive(Clone,Copy,Default,Debug,PartialEq,Eq)]
pub struct Line {
    start: usize,
    length: usize,
}

#[derive(Clone,Debug)]
pub struct ScrollbackBuffer {
    lines: CircularBuffer<Line>,
    cells: CircularBuffer<Cell>,
    lines_oldest_index: usize,
    total_lines: usize,
    cells_oldest_index: usize,
    total_cells: usize,
}

impl Default for ScrollbackBuffer {
    fn default() -> Self {
        let allocation_granularity = get_allocation_granularity();
        let lines = CircularBuffer::new(allocation_granularity).unwrap();
        let cells = CircularBuffer::new(allocation_granularity*128).unwrap();
        Self {
            lines,
            cells,
            lines_oldest_index: 0,
            total_lines: 0,
            cells_oldest_index: 0,
            total_cells: 0,
        }
    }
}

impl ScrollbackBuffer {
    pub fn get_lines(&self) -> &[Line] {
        &self.lines[self.lines_oldest_index..(self.lines_oldest_index+self.total_lines)]
    }

    pub fn get_row(&self, line: &Line) -> &[Cell] {
        &self.cells[line.start..(line.start+line.length)] 
    }

    pub fn extend_current_line(&mut self, src_buf: &[Cell]) {
        let chunk_length = self.cells.len();
        for chunk in src_buf.chunks(chunk_length) {
            self.extend_current_line_by_fittable_block(chunk);
        }
    }

    fn extend_current_line_by_fittable_block(&mut self, data: &[Cell]) {
        if self.total_lines == 0 {
            self.advance_line();
        }
        let start_cell_index = self.get_free_cell_index();
        self.push_and_clamp_into_current_line(data);
        self.evict_overridden_lines(start_cell_index, data.len());
    }

    pub fn advance_line(&mut self) {
        assert!(self.total_lines <= self.lines.len());
        if self.total_lines == self.lines.len() {
            self.lines[self.lines_oldest_index] =  Line::default();
            self.total_lines -= 1;
            self.lines_oldest_index = (self.lines_oldest_index + 1) % self.lines.len();
        }
        let line_index = self.get_free_line_index();
        let cell_index = self.get_free_cell_index();
        let line = &mut self.lines[line_index];
        line.start = cell_index;
        line.length = 0;
        self.total_lines += 1;
    }

    fn get_current_line_index(&self) -> usize {
        assert!(self.total_lines >= 1);
        let i = self.lines_oldest_index+self.total_lines-1;
        i % self.lines.len()
    }

    fn get_free_line_index(&self) -> usize {
        let i = self.lines_oldest_index+self.total_lines;
        i % self.lines.len()
    }

    fn get_free_cell_index(&self) -> usize {
        let i = self.cells_oldest_index+self.total_cells;
        i % self.cells.len()
    }

    fn push_and_clamp_into_current_line(&mut self, data: &[Cell]) {
        let start_cell_index = self.get_free_cell_index();
        let end_cell_index = start_cell_index + data.len();
        let dst_cells = &mut self.cells[start_cell_index..end_cell_index];
        dst_cells.copy_from_slice(data);
        // clamp current line 
        let line_index = self.get_current_line_index();
        let line = &mut self.lines[line_index];
        line.length += data.len();
        if line.length > self.cells.len() {
            let total_override = self.cells.len() - line.length;
            line.length = self.cells.len();
            line.start = (line.start + total_override) % self.cells.len();
        }
        // wrap cells
        self.total_cells += data.len();
        if self.total_cells > self.cells.len() {
            let total_override = self.total_cells - self.cells.len();
            self.total_cells = self.cells.len();
            self.cells_oldest_index = (self.cells_oldest_index + total_override) % self.cells.len();
        }
    }

    fn evict_overridden_lines(&mut self, start_cell_index: usize, total_cells: usize) {
        if self.total_lines == 0 {
            return;
        }
        let end_cell_index = start_cell_index + total_cells; // exclusive end of write region
        let start_line_index = self.lines_oldest_index;
        let end_line_index = start_line_index+self.total_lines-1;
        for line_index in start_line_index..end_line_index {
            let line = &mut self.lines[line_index];
            let mut start_old_index = line.start;
            // shift range of line so it is always within possible overlapping region
            if start_old_index < start_cell_index {
                start_old_index += self.cells.len();
            }
            if start_old_index >= end_cell_index {
                break;
            }
            // evict if start of the region lies within override range
            *line = Line::default();
            self.total_lines -= 1;
            self.lines_oldest_index = (self.lines_oldest_index + 1) % self.lines.len();
        }
    }
}
