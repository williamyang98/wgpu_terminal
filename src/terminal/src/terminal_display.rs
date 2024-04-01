use crate::{
    primitives::{Cell,StyleFlags, Pen},
    scrollback_buffer::ScrollbackBuffer,
};
use vt100::{
    common::{
        CursorStyle,
        Rgb8,
    },
};
use cgmath::Vector2;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct CursorStatus {
    pub is_visible: bool,
    pub is_blinking: bool,
    pub style: CursorStyle,
}

impl Default for CursorStatus {
    fn default() -> Self {
        Self {
            is_visible: true,
            is_blinking: true,
            style: CursorStyle::Block,
        }
    }
}

#[derive(Clone,Copy,Default,Debug)]
pub struct LineStatus {
    pub length: usize,
    pub is_linebreak: bool,
}

#[derive(Clone,Debug)]
pub struct TerminalDisplay {
    cursor: Vector2<usize>,
    size: Vector2<usize>,
    row_offset: usize,
    cells: Vec<Cell>,
    row_status: Vec<LineStatus>,
    scrollback_buffer: ScrollbackBuffer, // eject lines into scrollback buffer
    saved_cursor: Option<Vector2<usize>>,
    resize_cells: Vec<Cell>, // temporary resize buffers
    resize_row_status: Vec<LineStatus>,
    pub(crate) cursor_status: CursorStatus,
    pub(crate) pen: Pen,
    pub(crate) default_pen: Pen,
    pub(crate) is_newline_carriage_return: bool, // if true then \n will also set cursor.x = 0
}

pub const DEFAULT_VIEWPORT_SIZE: Vector2<usize> = Vector2::new(128,128);

impl Default for TerminalDisplay {
    fn default() -> Self {
        let total_cells = DEFAULT_VIEWPORT_SIZE.x * DEFAULT_VIEWPORT_SIZE.y;
        let default_pen = Pen {
            foreground_colour: Rgb8 { r: 255, b: 255, g: 255 },
            background_colour: Rgb8 { r: 5, b: 10, g: 7 },
            style_flags: StyleFlags::None,
        };
        let default_cell = Cell { pen: default_pen, character: ' ' };
        Self {
            cursor: Vector2::new(0,0),
            size: DEFAULT_VIEWPORT_SIZE,
            row_offset: 0,
            cells: vec![default_cell; total_cells],
            row_status: vec![LineStatus::default(); DEFAULT_VIEWPORT_SIZE.y],
            resize_cells: vec![default_cell; total_cells],
            resize_row_status: vec![LineStatus::default(); DEFAULT_VIEWPORT_SIZE.y],
            scrollback_buffer: ScrollbackBuffer::default(),
            cursor_status: CursorStatus::default(),
            saved_cursor: None,
            pen: default_pen,
            default_pen,
            is_newline_carriage_return: false,
        }
    }
}

impl TerminalDisplay {
    pub fn get_scrollback_buffer(&self) -> &ScrollbackBuffer {
        &self.scrollback_buffer
    }
 
    #[inline]
    pub(crate) fn write_utf8(&mut self, character: char) {
        let cell = Cell { character, pen: self.pen };
        self.write_cell(&cell);
    }

    #[inline]
    pub(crate) fn write_ascii(&mut self, b: u8) {
        match b {
            b'\n' => {
                self.feed_newline(true);
                if self.is_newline_carriage_return {
                    self.carriage_return();
                }
            }, 
            b'\r' => self.carriage_return(),
            b'\x08' => { 
                let mut cursor = self.get_cursor(); 
                cursor.x = cursor.x.saturating_sub(1);
                self.set_cursor(cursor);
            },
            b' '..=b'~' => { self.write_utf8(b as char); },
            b'\x07' => { log::info!("Ding ding ding (BELL)"); },
            b => { log::error!("Unhandled byte: {}", b); },
        }
    }

    pub(crate) fn save_cursor(&mut self) {
        self.saved_cursor = Some(self.cursor);
    }

    pub(crate) fn restore_cursor(&mut self) {
        match self.saved_cursor.take() {
            Some(cursor) => self.set_cursor(cursor),
            None => log::warn!("tried to restore nonexistent cursor from memory"),
        }
    }

    pub(crate) fn set_size(&mut self, new_size: Vector2<usize>) {
        assert!(new_size.x > 0);
        assert!(new_size.y > 0);
        if new_size == self.size {
            return;
        }
        let new_total_cells = new_size.x*new_size.y;
        let default_cell = Cell { pen: self.default_pen, character: ' ' };
        // copy into temporary buffer and reinsert them into resized grid
        let old_row_offset = self.row_offset;
        let old_cursor = self.cursor;
        let old_size = self.size;
        let old_total_cells = old_size.x*old_size.y;
        self.resize_cells.resize(old_total_cells, default_cell);
        self.resize_row_status.resize(old_size.y, LineStatus::default());
        self.resize_cells.copy_from_slice(self.cells.as_slice());
        self.resize_row_status.copy_from_slice(self.row_status.as_slice());
        // resize grid
        self.size = new_size;
        self.cells.resize(new_total_cells, default_cell);
        self.row_status.resize(new_size.y, LineStatus::default());
        // reset grid
        self.row_offset = 0;
        self.cursor = Vector2::new(0,0);
        self.cells.fill(default_cell);
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
                self.feed_newline(true);
                self.cursor.x = 0;
            }
        }
        // assume cursor wants to stay where it is
        self.set_cursor(old_cursor);
    }

    pub fn get_size(&self) -> Vector2<usize> {
        self.size
    }

    pub(crate) fn set_cursor(&mut self, cursor: Vector2<usize>) {
        // cursor can overflow the screen apparently without moving to new line
        // newline only occurs when a change is committed at the overflowing location onto the next line
        let cursor = Vector2::new(
            cursor.x.min(self.size.x),
            cursor.y.min(self.size.y-1),
        );
        self.cursor = cursor;
    }

    pub(crate) fn get_cursor(&self) -> Vector2<usize> {
        self.cursor
    }

    pub fn get_row(&self, row: usize) -> (&[Cell], &LineStatus) {
        assert!(row < self.size.y);
        let row = self.get_row_index(row);
        let i = self.size.x*row;
        let line = &self.cells[i..(i+self.size.x)];
        (line, &self.row_status[row])
    }
 
    pub(crate) fn get_row_mut(&mut self, row: usize) -> (&mut [Cell], &mut LineStatus) {
        assert!(row < self.size.y);
        let row = self.get_row_index(row);
        let i = self.size.x*row;
        let line = &mut self.cells[i..(i+self.size.x)];
        (line, &mut self.row_status[row])
    }
 
    #[inline]
    pub(crate) fn write_cell(&mut self, cell: &Cell) {
        self.wrap_cursor();
        let row = self.get_current_row_index();
        let line_status = &mut self.row_status[row];
        line_status.length = line_status.length.max(self.cursor.x+1);
        let index = row*self.size.x + self.cursor.x;
        self.cells[index] = *cell;
        self.cursor.x += 1;
    }

    #[inline]
    pub(crate) fn carriage_return(&mut self) {
        self.cursor.x = 0;
    }

    #[inline]
    pub(crate) fn feed_newline(&mut self, is_linebreak: bool) {
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

    #[inline]
    fn wrap_cursor(&mut self) {
        assert!(self.cursor.x <= self.size.x);
        if self.cursor.x == self.size.x {
            self.feed_newline(false);
            self.cursor.x = 0;
        }
    }

    #[inline]
    fn get_row_index(&self, row: usize) -> usize {
        (self.row_offset + row) % self.size.y
    }

    #[inline]
    fn get_current_row_index(&self) -> usize {
        (self.row_offset + self.cursor.y) % self.size.y
    }

    pub(crate) fn copy_lines_within(&mut self, src: usize, dst: usize, total: usize) {
        assert!(src < self.size.y);
        assert!(dst < self.size.y);
        let max_lines_to_copy = self.size.y - src.min(dst);
        assert!(total <= max_lines_to_copy);
        // copy backwards or forwards to avoid overriding lines in src range
        use std::cmp::Ordering;
        match src.cmp(&dst) {
            Ordering::Less => (0..total).rev().for_each(|i| self.copy_line_within(src+i, dst+i)),
            Ordering::Greater => (0..total).for_each(|i| self.copy_line_within(src+i, dst+i)),
            Ordering::Equal => {},
        }
    }

    pub(crate) fn copy_line_within(&mut self, src: usize, dst: usize) {
        assert!(src < self.size.y);
        assert!(dst < self.size.y);
        let row_src = self.get_row_index(src);
        let row_dst = self.get_row_index(dst);
        let index_src = row_src*self.size.x;
        let index_dst = row_dst*self.size.x;
        let slice_src = index_src..(index_src+self.size.x);
        self.cells.as_mut_slice().copy_within(slice_src, index_dst);
        self.row_status[row_dst] = self.row_status[row_src];
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
        let default_cell = Cell { character: ' ', pen: self.default_pen };
        line.fill(default_cell);
        *line_status = LineStatus::default();
        self.row_offset = (self.row_offset+1) % self.size.y;
    }
}
