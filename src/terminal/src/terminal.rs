use crate::{
    parser::Handler,
    utf8_parser::{
        ParserError as Utf8ParserError,
    },
    colour_table::{XTERM_COLOUR_TABLE, convert_u32_to_rgb},
};
use bitflags::bitflags;
use cgmath::Vector2;
use vt100::{
    parser::{
        Parser as Vt100Parser, 
        ParserError as Vt100ParserError,
    },
    command::Command as Vt100Command,
    misc::EraseMode,
    graphic_style::{Rgb8, GraphicStyle},
};

bitflags! {
    #[derive(Clone,Copy,Debug,Default)]
    pub struct CellFlags: u8 {
        const None          = 0b0000_0000;
        const Bold          = 0b0000_0001;
        const Dim           = 0b0000_0010;
        const Italic        = 0b0000_0100;
        const Underline     = 0b0000_1000;
        const Blinking      = 0b0001_0000;
        const Inverse       = 0b0010_0000;
        const Hidden        = 0b0100_0000;
        const Strikethrough = 0b1000_0000;
        const _ = 0u8;
    }
}

#[derive(Clone,Copy,Default)]
pub struct Cell {
    pub character: char,
    pub background_colour: Rgb8,
    pub foreground_colour: Rgb8,
    pub flags: CellFlags,
}

pub struct Terminal {
    cells: Vec<Cell>,
    size: Vector2<usize>,
    cursor: Vector2<usize>,
    is_cursor_visible: bool,
    foreground_colour: Rgb8,
    background_colour: Rgb8,
    cell_flags: CellFlags,
    default_foreground_colour: Rgb8,
    default_background_colour: Rgb8,
    colour_table: Vec<Rgb8>,
}

impl Terminal {
    pub fn new(size: Vector2<usize>) -> Self {
        let total_cells = size.x*size.y;
        let colour_table: Vec<Rgb8> = XTERM_COLOUR_TABLE
            .iter()
            .map(|v| {
                let mut rgb = convert_u32_to_rgb(*v);
                const A: u8 = 60;
                rgb.r = rgb.r.min(255-A) + A;
                rgb.g = rgb.g.min(255-A) + A;
                rgb.b = rgb.b.min(255-A) + A;
                rgb
            })
            .collect();
        assert!(colour_table.len() == 256);
        let default_foreground_colour = Rgb8 { r: 255, b: 255, g: 255 };
        let default_background_colour = Rgb8 { r: 0, b: 0, g: 0 };
        Self {
            cells: vec![Cell::default(); total_cells],
            size,
            cursor: Vector2::new(0,0),
            is_cursor_visible: false,
            foreground_colour: default_foreground_colour,
            background_colour: default_background_colour,
            cell_flags: CellFlags::None,
            default_foreground_colour,
            default_background_colour,
            colour_table,
        }
    }

    pub fn get_cells(&self) -> &'_ [Cell] {
        self.cells.as_slice()
    }

    pub fn get_size(&self) -> Vector2<usize> {
        self.size
    }

    pub fn resize(&mut self, size: Vector2<usize>) {
        let total_cells = size.x*size.y;
        self.size = size;
        self.set_cursor_position(self.cursor);
        self.cells.resize(total_cells, Cell::default());
    }

    fn write_ascii(&mut self, buf: &[u8]) {
        for &b in buf {
            if b == b'\n' {
                self.next_line_cursor();
            } else if b == b'\r' {
                self.cursor.x = 0;
            } else if b == b'\x08' {
                self.cursor.x = self.cursor.x.max(1) - 1;
            } else {
                self.write_cell(b as char);
            }
        }
    }

    fn write_cell(&mut self, character: char) {
        let index = self.cursor.x + self.cursor.y*self.size.x;
        let cell = &mut self.cells[index];
        cell.character = character;
        cell.foreground_colour = self.foreground_colour;
        cell.background_colour = self.background_colour;
        cell.flags = self.cell_flags;
        self.advance_cursor();
    }

    fn advance_cursor(&mut self) {
        self.cursor.x += 1; 
        if self.cursor.x >= self.size.x {
            self.cursor.x = 0;
            self.cursor.y += 1;
        }
        if self.cursor.y >= self.size.y {
            self.cursor.y = 0;
        }
    }

    fn next_line_cursor(&mut self) {
        self.cursor.x = 0;
        self.cursor.y += 1;
        if self.cursor.y >= self.size.y {
            self.cursor.y = 0;
        }
    }

    fn set_cursor_position(&mut self, pos: Vector2<usize>) {
        self.cursor.x = pos.x.min(self.size.x-1);
        self.cursor.y = pos.y.min(self.size.y-1);
    }

    fn set_graphic_styles(&mut self, styles: &[GraphicStyle]) {
        for &style in styles {
            match style {
                GraphicStyle::ResetAll => {
                    self.foreground_colour = self.default_foreground_colour;
                    self.background_colour = self.default_background_colour;
                    self.cell_flags = CellFlags::None;
                },
                // flags
                GraphicStyle::EnableBold => { self.cell_flags |= CellFlags::Bold; },
                GraphicStyle::EnableDim => { self.cell_flags |= CellFlags::Dim; },
                GraphicStyle::EnableItalic => { self.cell_flags |= CellFlags::Italic; },
                GraphicStyle::EnableUnderline => { self.cell_flags |= CellFlags::Underline; },
                GraphicStyle::EnableBlinking => { self.cell_flags |= CellFlags::Blinking; },
                GraphicStyle::EnableInverse => { self.cell_flags |= CellFlags::Inverse; },
                GraphicStyle::EnableHidden => { self.cell_flags |= CellFlags::Hidden; },
                GraphicStyle::EnableStrikethrough => { self.cell_flags |= CellFlags::Strikethrough; },
                GraphicStyle::DisableWeight => { self.cell_flags &= !(CellFlags::Bold | CellFlags::Dim); },
                GraphicStyle::DisableItalic => { self.cell_flags &= !CellFlags::Italic; },
                GraphicStyle::DisableUnderline => { self.cell_flags &= !CellFlags::Underline; },
                GraphicStyle::DisableBlinking => { self.cell_flags &= !CellFlags::Blinking; },
                GraphicStyle::DisableInverse => { self.cell_flags &= !CellFlags::Inverse; },
                GraphicStyle::DisableHidden => { self.cell_flags &= !CellFlags::Hidden; },
                GraphicStyle::DisableStrikethrough => { self.cell_flags &= !CellFlags::Strikethrough; },
                // foreground colours
                GraphicStyle::ForegroundBlack => { self.foreground_colour = self.colour_table[0]; },
                GraphicStyle::ForegroundRed => { self.foreground_colour = self.colour_table[1]; },
                GraphicStyle::ForegroundGreen => { self.foreground_colour = self.colour_table[2]; },
                GraphicStyle::ForegroundYellow => { self.foreground_colour = self.colour_table[3]; },
                GraphicStyle::ForegroundBlue => { self.foreground_colour = self.colour_table[4]; },
                GraphicStyle::ForegroundMagenta => { self.foreground_colour = self.colour_table[5]; },
                GraphicStyle::ForegroundCyan => { self.foreground_colour = self.colour_table[6]; },
                GraphicStyle::ForegroundWhite => { self.foreground_colour = self.colour_table[7]; },
                GraphicStyle::ForegroundExtended => { log::info!("[vt100] GraphicStyle({:?})", style); },
                GraphicStyle::ForegroundDefault => { self.foreground_colour = self.default_foreground_colour; },
                // background colours
                GraphicStyle::BackgroundBlack => { self.background_colour = self.colour_table[0]; },
                GraphicStyle::BackgroundRed => { self.background_colour = self.colour_table[1]; },
                GraphicStyle::BackgroundGreen => { self.background_colour = self.colour_table[2]; },
                GraphicStyle::BackgroundYellow => { self.background_colour = self.colour_table[3]; },
                GraphicStyle::BackgroundBlue => { self.background_colour = self.colour_table[4]; },
                GraphicStyle::BackgroundMagenta => { self.background_colour = self.colour_table[5]; },
                GraphicStyle::BackgroundCyan => { self.background_colour = self.colour_table[6]; },
                GraphicStyle::BackgroundWhite => { self.background_colour = self.colour_table[7]; },
                GraphicStyle::BackgroundExtended => { log::info!("[vt100] GraphicStyle({:?})", style); },
                GraphicStyle::BackgroundDefault => { self.background_colour = self.default_background_colour; },
                // bright foreground colours
                GraphicStyle::BrightForegroundBlack => { self.foreground_colour = self.colour_table[0]; },
                GraphicStyle::BrightForegroundRed => { self.foreground_colour = self.colour_table[1]; },
                GraphicStyle::BrightForegroundGreen => { self.foreground_colour = self.colour_table[2]; },
                GraphicStyle::BrightForegroundYellow => { self.foreground_colour = self.colour_table[3]; },
                GraphicStyle::BrightForegroundBlue => { self.foreground_colour = self.colour_table[4]; },
                GraphicStyle::BrightForegroundMagenta => { self.foreground_colour = self.colour_table[5]; },
                GraphicStyle::BrightForegroundCyan => { self.foreground_colour = self.colour_table[6]; },
                GraphicStyle::BrightForegroundWhite => { self.foreground_colour = self.colour_table[7]; },
                // bright background colours
                GraphicStyle::BrightBackgroundBlack => { self.background_colour = self.colour_table[0]; },
                GraphicStyle::BrightBackgroundRed => { self.background_colour = self.colour_table[1]; },
                GraphicStyle::BrightBackgroundGreen => { self.background_colour = self.colour_table[2]; },
                GraphicStyle::BrightBackgroundYellow => { self.background_colour = self.colour_table[3]; },
                GraphicStyle::BrightBackgroundBlue => { self.background_colour = self.colour_table[4]; },
                GraphicStyle::BrightBackgroundMagenta => { self.background_colour = self.colour_table[5]; },
                GraphicStyle::BrightBackgroundCyan => { self.background_colour = self.colour_table[6]; },
                GraphicStyle::BrightBackgroundWhite => { self.background_colour = self.colour_table[7]; },
            }
        }
    }
}

impl Handler for Terminal {
    fn on_ascii_data(&mut self, buf: &[u8]) {
        self.write_ascii(buf);
    }

    fn on_utf8(&mut self, character: char) {
        self.write_cell(character);
    }

    fn on_unhandled_byte(&mut self, byte: u8) {
        log::error!("[unknown-byte] ({:?})", byte);
    }

    fn on_utf8_error(&mut self, error: &Utf8ParserError) {
        log::error!("[utf8-error] {:?}", error);
    }

    fn on_vt100(&mut self, c: &Vt100Command) {
        match c {
            Vt100Command::SetWindowTitle(data) => match std::str::from_utf8(data) {
                Ok(title) => log::info!("[vt100] SetWindowTitleUtf8('{}')", title),
                Err(_err) => log::info!("[vt100] SetWindowTitleBytes({:?})", data),
            },
            Vt100Command::SetBackgroundColourRgb(ref rgb) => {
                self.background_colour = *rgb;
            },
            Vt100Command::SetForegroundColourRgb(ref rgb) => {
                self.foreground_colour = *rgb;
            },
            Vt100Command::SetBackgroundColourTable(index) => {
                self.background_colour = self.colour_table[*index as usize];
            },
            Vt100Command::SetForegroundColourTable(index) => {
                self.foreground_colour = self.colour_table[*index as usize];
            },
            Vt100Command::MoveCursorPositionViewport(ref pos) => {
                // top left corner is (1,1)
                let pos = Vector2::new((pos.x.get()-1) as usize, (pos.y.get()-1) as usize);
                self.set_cursor_position(pos);
            },
            Vt100Command::EraseInDisplay(mode) => match mode {
                EraseMode::FromCursorToEnd => {
                    let i = self.cursor.x + self.cursor.y*self.size.x;
                    self.cells[i..].fill(Cell::default());
                },
                EraseMode::FromCursorToStart => {
                    let i = self.cursor.x + self.cursor.y*self.size.x;
                    self.cells[..=i].fill(Cell::default());
                },
                EraseMode::EntireDisplay => {
                    self.cells.fill(Cell::default());
                },
                EraseMode::SavedLines => {
                    self.cells.fill(Cell::default());
                },
            },
            Vt100Command::EraseInLine(mode) => match mode {
                EraseMode::FromCursorToEnd => {
                    let row = self.cursor.y*self.size.x;
                    let i_start = row + self.cursor.x;
                    let i_end = row + self.size.x;
                    self.cells[i_start..i_end].fill(Cell::default());
                },
                EraseMode::FromCursorToStart => {
                    let i_start = self.cursor.y*self.size.x;
                    let i_end = i_start + self.cursor.x;
                    self.cells[i_start..=i_end].fill(Cell::default());
                },
                EraseMode::EntireDisplay => {
                    let i_start = self.cursor.y*self.size.x;
                    let i_end = i_start + self.size.x;
                    self.cells[i_start..i_end].fill(Cell::default());
                },
                EraseMode::SavedLines => {
                    let i_start = self.cursor.y*self.size.x;
                    let i_end = i_start + self.size.x;
                    self.cells[i_start..=i_end].fill(Cell::default());
                },
            },
            Vt100Command::ReplaceWithSpaces(total) => {
                let row = self.cursor.y*self.size.x;
                let i_start = row + self.cursor.x;
                let i_max = row + self.size.x;
                let i_end = (i_start + total.get() as usize).min(i_max);
                self.cells[i_start..i_end].fill(Cell::default());
            },
            Vt100Command::MoveCursorRight(total) => {
                self.cursor.x = (self.cursor.x + total.get() as usize).min(self.size.x-1);
            }
            Vt100Command::HideCursor => {
                self.is_cursor_visible = false;
            },
            Vt100Command::ShowCursor => {
                self.is_cursor_visible = true;
            },
            Vt100Command::SetGraphicStyles(ref styles) => {
                self.set_graphic_styles(styles);
            },
            Vt100Command::SetHyperlink { tag, link } => {
                let tag_res = std::str::from_utf8(tag);
                let link_res = std::str::from_utf8(link);
                match (tag_res, link_res) {
                    (Ok(tag), Ok(link)) => log::info!("[vt100] SetHyperlink(tag: '{}', link: '{}')", tag, link), 
                    (Err(_), Ok(link)) => log::info!("[vt100] SetHyperlink(tag: '{:?}', link: '{}')", tag, link), 
                    (Ok(tag), Err(_)) => log::info!("[vt100] SetHyperlink(tag: '{}', link: '{:?}')", tag, link), 
                    (Err(_), Err(_)) => log::info!("[vt100] SetHyperlink(tag: '{:?}', link: '{:?}')", tag, link), 
                }
            },
            c => log::info!("[vt100] ({:?})", c),
        }
    }

    fn on_vt100_error(&mut self, err: &Vt100ParserError, parser: &Vt100Parser) {
        log::error!("[vt100-error] {:?} {:?}", err, parser);
    }
}
