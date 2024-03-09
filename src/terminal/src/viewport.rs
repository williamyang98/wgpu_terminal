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
    resize_cells: Vec<Cell>,
    resize_row_status: Vec<LineStatus>,
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
            resize_cells: vec![Cell::default(); total_cells],
            resize_row_status: vec![LineStatus::default(); DEFAULT_VIEWPORT_SIZE.y],
            scrollback_buffer: ScrollbackBuffer::default(),
        }
    }
}

impl Viewport {
    pub fn get_scrollback_buffer(&self) -> &ScrollbackBuffer {
        &self.scrollback_buffer
    }

    pub fn set_size(&mut self, new_size: Vector2<usize>) {
        assert!(new_size.x > 1);
        assert!(new_size.y > 0);
        if new_size == self.size {
            return;
        }
        let new_total_cells = new_size.x*new_size.y;
        // copy into temporary buffer and reinsert them into resized grid
        let old_row_offset = self.row_offset;
        let old_cursor = self.cursor;
        let old_size = self.size;
        let old_total_cells = old_size.x*old_size.y;
        self.resize_cells.resize(old_total_cells, Cell::default());
        self.resize_row_status.resize(old_size.y, LineStatus::default());
        self.resize_cells.copy_from_slice(self.cells.as_slice());
        self.resize_row_status.copy_from_slice(self.row_status.as_slice());
        // resize grid
        self.size = new_size;
        self.cells.resize(new_total_cells, Cell::default());
        self.row_status.resize(new_size.y, LineStatus::default());
        // reset grid
        self.row_offset = 0;
        self.cursor = Vector2::new(0,0);
        self.cells.fill(Cell::default());
        self.row_status.fill(LineStatus::default());
        // reinsert
        for row_index in 0..old_size.y {
            let row_index = (row_index + old_row_offset) % old_size.y;
            let line = self.resize_row_status[row_index];
            if line.length == 0 && !line.is_linebreak {
                break;
            }
            for col_index in 0..line.length {
                let index = row_index*old_size.x + col_index;
                let cell = self.resize_cells[index];
                self.write_cell(&cell);
            }
            if line.is_linebreak {
                self.next_line_cursor(true);
            }
        }
        // assume cursor wants to stay where it is
        self.set_cursor(old_cursor);
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
            b' '..=b'~' => { self.write_utf8(b as char); },
            b'\x07' => { log::info!("Ding ding ding (BELL)"); },
            b => { log::error!("Unhandled byte: {}", b); },
        }
    }

    pub fn write_utf8(&mut self, character: char) {
        let mut cell = Cell::default();
        cell.character = character;
        cell.colour_from_pen(&self.pen);
        self.write_cell(&cell);
    }

    fn write_cell(&mut self, cell: &Cell) {
        self.wrap_cursor();
        let row = self.get_current_row_index();
        let line_status = &mut self.row_status[row];
        line_status.length = line_status.length.max(self.cursor.x+1);
        let index = row*self.size.x + self.cursor.x;
        self.cells[index] = *cell;
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
