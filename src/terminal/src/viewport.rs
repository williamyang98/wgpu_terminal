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

#[derive(Clone,Debug)]
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
    is_newline_carriage_return: bool, // if true then \n will also set cursor.x = 0
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
            is_newline_carriage_return: false,
        }
    }
}

impl Viewport {
    pub fn get_scrollback_buffer(&self) -> &ScrollbackBuffer {
        &self.scrollback_buffer
    }
 
    pub fn set_is_newline_carriage_return(&mut self, v: bool) {
        self.is_newline_carriage_return = v;
    }

    pub fn set_size(&mut self, new_size: Vector2<usize>) {
        assert!(new_size.x > 0);
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
                self.cursor.x = 0;
            }
        }
        // assume cursor wants to stay where it is
        self.set_cursor(old_cursor);
    }

    pub fn get_size(&self) -> Vector2<usize> {
        self.size
    }

    pub fn set_cursor(&mut self, cursor: Vector2<usize>) {
        // cursor can overflow the screen apparently without moving to new line
        // newline only occurs when a change is committed at the overflowing location onto the next line
        let cursor = Vector2::new(
            cursor.x.min(self.size.x),
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
 
    pub fn get_pen(&self) -> &Pen {
        &self.pen
    }

    pub fn get_pen_mut(&mut self) -> &mut Pen {
        &mut self.pen
    }

    pub fn write_ascii(&mut self, b: u8) {
        match b {
            b'\n' => {
                self.next_line_cursor(true);
                if self.is_newline_carriage_return {
                    self.cursor.x = 0;
                }
            }, 
            b'\x0a' => self.next_line_cursor(true), // raw linefeed
            b'\r' => { self.cursor.x = 0; },
            b'\x08' => { self.cursor.x = self.cursor.x.max(1) - 1; },
            b' '..=b'~' => { self.write_utf8(b as char); },
            b'\x07' => { log::info!("Ding ding ding (BELL)"); },
            b => { log::error!("Unhandled byte: {}", b); },
        }
    }

    pub fn write_utf8(&mut self, character: char) {
        let mut cell = Cell {
            character,
            ..Cell::default()
        };
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
            self.cursor.x = 0;
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
        self.cursor.y += 1;
        if self.cursor.y == self.size.y {
            self.eject_oldest_line_into_scrollbuffer();
            self.cursor.y = self.size.y-1;
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
        self.row_offset = (self.row_offset+1) % self.size.y;
    }
 
    pub fn scroll_up(&mut self) {
        self.eject_oldest_line_into_scrollbuffer();
    }

    pub fn scroll_down(&mut self) {
        // move lines down and fill with nothing
        self.row_offset = (self.row_offset+self.size.y-1) % self.size.y;
        let clear_row = self.row_offset;
        let index = self.size.x*clear_row;
        let line = &mut self.cells[index..(index+self.size.x)];
        let line_status = &mut self.row_status[clear_row];
        line.fill(Cell::default());
        *line_status = LineStatus {
            is_linebreak: true,
            ..LineStatus::default()
        };
    }

    pub fn insert_lines(&mut self, total: usize) {
        // cursor is 0 <= x <= Nx, 0 <= y < Ny
        let max_rows = self.size.y - self.cursor.y;
        let total = total.min(max_rows);
        let shift = max_rows-total;
        // shift rows downwards
        for i in (0..shift).rev() {
            let src_row = i;
            let dst_row = i + total;
            let src_row_index = (self.row_offset + self.cursor.y + src_row) % self.size.y;
            let dst_row_index = (self.row_offset + self.cursor.y + dst_row) % self.size.y;
            self.row_status[dst_row_index] = self.row_status[src_row_index]; 
            let src_cell_offset = src_row_index*self.size.x;
            let dst_cell_offset = dst_row_index*self.size.x;
            for col in 0..self.size.x {
                let dst_cell_index = dst_cell_offset + col;
                let src_cell_index = src_cell_offset + col;
                self.cells[dst_cell_index] = self.cells[src_cell_index];
            }
        }
        for i in 0..total {
            let row = (self.row_offset + self.cursor.y + i) % self.size.y;
            self.row_status[row] = LineStatus {
                is_linebreak: true,
                ..LineStatus::default()
            };
            let cell_offset = row*self.size.x;
            let row = &mut self.cells[cell_offset..(cell_offset+self.size.x)];
            row.fill(Cell::default());
        }
    }

    pub fn delete_lines(&mut self, total: usize) {
        // cursor is 0 <= x <= Nx, 0 <= y < Ny
        let max_rows = self.size.y - self.cursor.y;
        let total = total.min(max_rows);
        let shift = max_rows-total;
        // shift rows upwards
        for i in 0..shift {
            let src_row = i + total;
            let dst_row = i;
            let src_row_index = (self.row_offset + self.cursor.y + src_row) % self.size.y;
            let dst_row_index = (self.row_offset + self.cursor.y + dst_row) % self.size.y;
            self.row_status[dst_row_index] = self.row_status[src_row_index]; 
            let src_cell_offset = src_row_index*self.size.x;
            let dst_cell_offset = dst_row_index*self.size.x;
            for col in 0..self.size.x {
                let dst_cell_index = dst_cell_offset + col;
                let src_cell_index = src_cell_offset + col;
                self.cells[dst_cell_index] = self.cells[src_cell_index];
            }
        }
        for i in 0..total {
            let j = i + shift;
            let row = (self.row_offset + self.cursor.y + j) % self.size.y;
            self.row_status[row] = LineStatus {
                is_linebreak: false,
                ..LineStatus::default()
            };
            let cell_offset = row*self.size.x;
            let row = &mut self.cells[cell_offset..(cell_offset+self.size.x)];
            row.fill(Cell::default());
        }
    }
}
