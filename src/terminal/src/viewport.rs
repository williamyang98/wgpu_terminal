use crate::{
    primitives::{Cell,Pen},
    scrollback_buffer::ScrollbackBuffer,
};
use cgmath::Vector2;

#[derive(Clone,Copy,Default,Debug)]
pub struct LineStatus {
    pub length: usize,
    pub is_linebreak: bool,
}

pub struct Viewport {
    cursor: Vector2<usize>,
    size: Vector2<usize>,
    row_offset: usize,
    pen: Pen,
    cells: Vec<Cell>,
    row_status: Vec<LineStatus>,
    _old_cells: Vec<Cell>, // when we resize we swap and copy
    _old_row_status: Vec<LineStatus>,
    scrollback_buffer: ScrollbackBuffer, // eject lines into scrollback buffer
}

pub const DEFAULT_VIEWPORT_SIZE: Vector2<usize> = Vector2::new(128,128);

impl Default for Viewport {
    fn default() -> Self {
        let total_cells = DEFAULT_VIEWPORT_SIZE.x * DEFAULT_VIEWPORT_SIZE.y;
        Self {
            cursor: Vector2::new(0,0),
            size: DEFAULT_VIEWPORT_SIZE,
            row_offset: 0,
            pen: Pen::default(),
            cells: vec![Cell::default(); total_cells],
            row_status: vec![LineStatus::default(); DEFAULT_VIEWPORT_SIZE.y],
            _old_cells: vec![Cell::default(); total_cells],
            _old_row_status: vec![LineStatus::default(); DEFAULT_VIEWPORT_SIZE.y],
            scrollback_buffer: ScrollbackBuffer::default(),
        }
    }
}

impl Viewport {
    pub fn get_scrollback_buffer(&self) -> &ScrollbackBuffer {
        &self.scrollback_buffer
    }

    pub fn set_size(&mut self, size: Vector2<usize>) {
        assert!(size.x > 1);
        assert!(size.y > 0);
        if size == self.size {
            return;
        }
        let total_cells = size.x*size.y;
        self._old_cells.resize(total_cells, Cell::default());
        self._old_cells.fill(Cell::default());
        self._old_row_status.resize(size.y, LineStatus::default());
        self._old_row_status.fill(LineStatus::default());
        // copy by rerendering each line including breaks or wrapping if it doesnt fit
        let mut dst_cursor: Vector2<usize> = Vector2::new(0,0);
        for row_index in 0..self.size.y {
            let src_row_index = (self.row_offset + row_index) % self.size.y;
            let src_index_offset = self.size.x*src_row_index;
            let src_line = &self.cells[src_index_offset..(src_index_offset+self.size.x)];
            let src_status = &self.row_status[src_row_index];
            // copy as many cells as we can while doing line rewrapping 
            for col_index in 0..src_status.length {
                assert!(dst_cursor.x < size.x);
                assert!(dst_cursor.y < size.y);
                let dst_index_offset = size.x*dst_cursor.y;
                let src_cell = &src_line[col_index];
                let dst_cell = &mut self._old_cells[dst_index_offset+dst_cursor.x];
                let dst_status = &mut self._old_row_status[dst_cursor.y];
                *dst_cell = *src_cell; 
                dst_status.length += 1;
                dst_cursor.x += 1;
                if dst_cursor.x < size.x {
                    continue;
                }
                dst_cursor.x = 0;
                dst_cursor.y += 1;
                if dst_cursor.y >= size.y {
                    break;
                }
            }
            // early full write
            if dst_cursor.y >= size.y {
                break;
            }
            // generate line break
            if src_status.is_linebreak {
                let dst_status = &mut self._old_row_status[dst_cursor.y];
                dst_status.is_linebreak = true;
                dst_cursor.x = 0;
                dst_cursor.y += 1;
            }
            if dst_cursor.y >= size.y {
                break;
            }
        } 
        // swap in resized data
        self.row_offset = 0;
        std::mem::swap(&mut self.cells, &mut self._old_cells);
        std::mem::swap(&mut self.row_status, &mut self._old_row_status);
        self.size = size;
        self.set_cursor(self.cursor);
    }

    pub fn get_size(&self) -> Vector2<usize> {
        self.size
    }

    pub fn set_cursor(&mut self, cursor: Vector2<usize>) {
        let cursor = Vector2::new(
            cursor.x.min(self.size.x-1),
            cursor.y.min(self.size.y-1),
        );
        self.cursor = cursor;
    }

    pub fn get_cursor(&self) -> Vector2<usize> {
        self.cursor
    }

    pub fn get_row(&self, row: usize) -> (&[Cell], &LineStatus) {
        assert!(row < self.size.y);
        let row = self.get_row_index(row);
        let i = self.size.x*row;
        let line = &self.cells[i..(i+self.size.x)];
        (line, &self.row_status[row])
    }
 
    pub fn get_row_mut(&mut self, row: usize) -> (&mut [Cell], &mut LineStatus) {
        assert!(row < self.size.y);
        let row = self.get_row_index(row);
        let i = self.size.x*row;
        let line = &mut self.cells[i..(i+self.size.x)];
        (line, &mut self.row_status[row])
    }

    pub fn get_pen_mut(&mut self) -> &mut Pen {
        &mut self.pen
    }

    pub fn write_ascii(&mut self, b: u8) {
        match b {
            b'\n' => { self.next_line_cursor(true); }, 
            b'\r' => { self.cursor.x = 0; },
            b'\x08' => { self.cursor.x = self.cursor.x.max(1) - 1; },
            b' '..=b'~' => { self.write_cell(b as char); },
            b'\x07' => { log::info!("Ding ding ding (BELL)"); },
            b => { log::error!("Unhandled byte: {}", b); },
        }
    }

    pub fn write_cell(&mut self, character: char) {
        self.wrap_cursor();
        let row = self.get_current_row_index();
        let line_status = &mut self.row_status[row];
        line_status.length = line_status.length.max(self.cursor.x+1);
        let index = row*self.size.x + self.cursor.x;
        let cell = &mut self.cells[index];
        cell.character = character;
        cell.colour_from_pen(&self.pen);
        self.cursor.x += 1;
    }

    fn wrap_cursor(&mut self) {
        assert!(self.cursor.x <= self.size.x);
        if self.cursor.x == self.size.x {
            let is_linebreak = false;
            self.next_line_cursor(is_linebreak);
        }
    }

    fn next_line_cursor(&mut self, is_linebreak: bool) {
        assert!(self.cursor.y < self.size.y);
        assert!(self.row_offset < self.size.y);
        {
            let curr_row = self.get_current_row_index();
            let line_status = &mut self.row_status[curr_row];
            if is_linebreak {
                line_status.is_linebreak = true;
            }
        }
        // advance cursor
        self.cursor.x = 0;
        self.cursor.y += 1;
        if self.cursor.y == self.size.y {
            self.eject_oldest_line_into_scrollbuffer();
            self.cursor.y = self.size.y-1;
            self.row_offset = (self.row_offset+1) % self.size.y;
        }
    }

    fn get_row_index(&self, row: usize) -> usize {
        (self.row_offset + row) % self.size.y
    }

    fn get_current_row_index(&self) -> usize {
        (self.row_offset + self.cursor.y) % self.size.y
    }

    fn eject_oldest_line_into_scrollbuffer(&mut self) {
        let eject_row = self.row_offset;
        let index = self.size.x*eject_row;
        let line = &mut self.cells[index..(index+self.size.x)];
        let line_status = &mut self.row_status[eject_row];
        self.scrollback_buffer.extend_current_line(&line[..line_status.length]);
        if line_status.is_linebreak {
            self.scrollback_buffer.advance_line();
        }
        line.fill(Cell::default());
        *line_status = LineStatus::default();
    }

}
