use cgmath::Vector2;
use terminal::{
    terminal::Terminal,
    primitives::Cell,
};

#[derive(Clone,Copy,Default,Debug)]
pub enum ReadMode {
    #[default]
    Bottom,
    Floating(usize),
}

pub struct TerminalReader {
    cells: Vec<Cell>,
    mode: ReadMode,
    size: Vector2<usize>,
    last_known_total_rows: usize,
}

impl Default for TerminalReader {
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            mode: ReadMode::Bottom,
            size: Vector2::new(0,0),
            last_known_total_rows: 0,
        }
    }
}

impl TerminalReader {
    pub fn set_size(&mut self, size: Vector2<usize>) {
        let total_cells = size.x*size.y;
        self.size = size;
        self.cells.resize(total_cells, Cell::default());
    }

    pub fn get_size(&self) -> Vector2<usize> {
        self.size
    }

    pub fn get_cells(&self) -> &[Cell] {
        self.cells.as_slice()
    }

    pub fn read_terminal(&mut self, terminal: &Terminal) {
        let viewport = terminal.get_viewport();
        let scrollback_buffer = viewport.get_scrollback_buffer();
        let scrollback_buffer_lines = scrollback_buffer.get_lines();
        let size = viewport.get_size();

        self.set_size(size); 
        self.last_known_total_rows = scrollback_buffer_lines.len();
        let scrollback_row = match self.mode {
            ReadMode::Bottom => scrollback_buffer_lines.len(),
            ReadMode::Floating(row) => {
                if row >= scrollback_buffer_lines.len() {
                    self.mode = ReadMode::Bottom;
                    scrollback_buffer_lines.len()
                } else {
                    row
                }
            },
        };
        self.cells.fill(Cell::default());

        let mut cursor: Vector2<usize> = Vector2::new(0,0);
        // render scrollback buffer
        for line in &scrollback_buffer_lines[scrollback_row..] {
            if cursor.y >= size.y {
                break;
            }
            let row = scrollback_buffer.get_row(line);
            for cell in row {
                if cursor.x >= size.x {
                    cursor.x = 0;
                    cursor.y += 1;
                }
                if cursor.y >= size.y {
                    break;
                }
                let dst_index = cursor.y*size.x + cursor.x;
                self.cells[dst_index] = *cell;
                cursor.x += 1;
            }
            if cursor.y >= size.y {
                break;
            }
            cursor.x = 0;
            cursor.y += 1;
        }
        // render viewport
        for y in 0..size.y {
            if cursor.y >= size.y {
                break;
            }
            let (src_row, status) = viewport.get_row(y);
            assert!(status.length <= size.x);
            let dst_index = cursor.y*size.x;
            let dst_row = &mut self.cells[dst_index..(dst_index+status.length)];
            dst_row.copy_from_slice(&src_row[..status.length]);
            cursor.y += 1;
        }
    }

    pub fn scroll_up(&mut self, total: usize) {
        let mode = match self.mode {
            ReadMode::Bottom => {
                let new_row = self.last_known_total_rows.saturating_sub(total);
                ReadMode::Floating(new_row)
            },
            ReadMode::Floating(row) => {
                let new_row = row.saturating_sub(total);
                ReadMode::Floating(new_row)
            },
        };
        self.mode = mode;
    }

    pub fn scroll_down(&mut self, total: usize) {
        let mode = match self.mode {
            ReadMode::Bottom => {
                ReadMode::Bottom
            },
            ReadMode::Floating(row) => {
                let new_row = row + total;
                if new_row >= self.last_known_total_rows {
                    ReadMode::Bottom
                } else {
                    ReadMode::Floating(new_row)
                }
            },
        };
        self.mode = mode;
    }

    pub fn scroll_to_top(&mut self) {
        self.mode = ReadMode::Floating(0);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.mode = ReadMode::Bottom;
    }
}
