use crate::terminal_display::TerminalDisplay;
use crate::primitives::Cell;
use cgmath::Vector2;

#[derive(Clone,Copy,Default,Debug)]
pub enum RenderPosition {
    #[default]
    Bottom,
    Floating(usize),
}

pub struct TerminalRenderer {
    cells: Vec<Cell>,
    size: Vector2<usize>,
    position: RenderPosition,
    last_known_total_rows: usize,
}

impl Default for TerminalRenderer {
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            size: Vector2::new(0,0),
            position: RenderPosition::Bottom,
            last_known_total_rows: 0,
        }
    }
}

impl TerminalRenderer {
    pub fn get_size(&self) -> Vector2<usize> {
        self.size
    }

    pub fn get_cells(&self) -> &[Cell] {
        self.cells.as_slice()
    }

    fn set_size(&mut self, size: Vector2<usize>) {
        let total_cells = size.x*size.y;
        self.size = size;
        self.cells.resize(total_cells, Cell::default());
    }

    pub fn render_display(&mut self, display: &TerminalDisplay) {
        let viewport = display.get_current_viewport();
        let size = viewport.get_size();
        self.set_size(size);

        let default_pen = viewport.default_pen;
        let default_cell = Cell { character: ' ', pen: default_pen };
        self.cells.fill(default_cell);

        let mut cursor: Vector2<usize> = Vector2::new(0,0);
 
        if let Some(scrollback_buffer) = viewport.scrollback_buffer.as_ref() {
            let scrollback_buffer_lines = scrollback_buffer.get_lines();
            self.last_known_total_rows = scrollback_buffer_lines.len();
            let scrollback_row = match self.position {
                RenderPosition::Bottom => scrollback_buffer_lines.len(),
                RenderPosition::Floating(row) => {
                    if row >= scrollback_buffer_lines.len() {
                        self.position = RenderPosition::Bottom;
                        scrollback_buffer_lines.len()
                    } else {
                        row
                    }
                },
            };

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
        }
        // render viewport
        let viewport_offset = cursor;
        let viewport_cursor = viewport.get_cursor();
        for y in 0..size.y {
            if cursor.y >= size.y {
                break;
            }
            let (src_row, status) = viewport.get_row(y);
            assert!(status.length <= size.x);
            let dst_index = cursor.y*size.x;
            let dst_row = &mut self.cells[dst_index..(dst_index+size.x)];
            dst_row[..status.length].copy_from_slice(&src_row[..status.length]);
            dst_row[status.length..].iter_mut().for_each(|c| {
                c.character = ' ';
                c.pen = default_pen;
            });
            cursor.y += 1;
        }

        let display_cursor = viewport_offset + viewport_cursor;
        let display_cursor_index = display_cursor.y*size.x + display_cursor.x;
        if let Some(cell) = self.cells.get_mut(display_cursor_index) {
            // TODO: render cursor properly with all the different styles
            std::mem::swap(&mut cell.pen.foreground_colour, &mut cell.pen.background_colour);
        }
    }

    pub fn scroll_up(&mut self, total: usize) {
        let position = match self.position {
            RenderPosition::Bottom => {
                let new_row = self.last_known_total_rows.saturating_sub(total);
                RenderPosition::Floating(new_row)
            },
            RenderPosition::Floating(row) => {
                let new_row = row.saturating_sub(total);
                RenderPosition::Floating(new_row)
            },
        };
        self.position = position;
    }

    pub fn scroll_down(&mut self, total: usize) {
        let position = match self.position {
            RenderPosition::Bottom => {
                RenderPosition::Bottom
            },
            RenderPosition::Floating(row) => {
                let new_row = row + total;
                if new_row >= self.last_known_total_rows {
                    RenderPosition::Bottom
                } else {
                    RenderPosition::Floating(new_row)
                }
            },
        };
        self.position = position;
    }

    pub fn scroll_to_top(&mut self) {
        self.position = RenderPosition::Floating(0);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.position = RenderPosition::Bottom;
    }
}
