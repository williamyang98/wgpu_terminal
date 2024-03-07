use crate::{
    primitives::{Cell, StyleFlags, Pen},
    parser::Handler,
    utf8_parser::ParserError as Utf8ParserError,
    colour_table::{XTERM_COLOUR_TABLE, convert_u32_to_rgb},
    viewport::{Viewport, LineStatus},
};
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

pub struct Terminal {
    viewport: Viewport,
    is_cursor_visible: bool,
    default_pen: Pen,
    colour_table: Vec<Rgb8>,
}

impl Default for Terminal {
    fn default() -> Self {
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
        let default_pen = Pen {
            foreground_colour: Rgb8 { r: 255, b: 255, g: 255 },
            background_colour: Rgb8 { r: 0, b: 0, g: 0 },
            style_flags: StyleFlags::None,
        };
        let mut res = Self {
            viewport: Viewport::default(),
            is_cursor_visible: false,
            default_pen,
            colour_table,
        };
        *res.viewport.get_pen_mut() = res.default_pen;
        res
    }
}

impl Terminal {
    pub fn get_viewport(&self) -> &Viewport {
        &self.viewport
    }

    pub fn get_viewport_mut(&mut self) -> &mut Viewport {
        &mut self.viewport
    }

    fn set_graphic_styles(&mut self, styles: &[GraphicStyle]) {
        let pen = self.viewport.get_pen_mut();
        for &style in styles {
            match style {
                GraphicStyle::ResetAll => {
                    *pen = self.default_pen;
                },
                // flags
                GraphicStyle::EnableBold => { pen.style_flags |= StyleFlags::Bold; },
                GraphicStyle::EnableDim => { pen.style_flags |= StyleFlags::Dim; },
                GraphicStyle::EnableItalic => { pen.style_flags |= StyleFlags::Italic; },
                GraphicStyle::EnableUnderline => { pen.style_flags |= StyleFlags::Underline; },
                GraphicStyle::EnableBlinking => { pen.style_flags |= StyleFlags::Blinking; },
                GraphicStyle::EnableInverse => { pen.style_flags |= StyleFlags::Inverse; },
                GraphicStyle::EnableHidden => { pen.style_flags |= StyleFlags::Hidden; },
                GraphicStyle::EnableStrikethrough => { pen.style_flags |= StyleFlags::Strikethrough; },
                GraphicStyle::DisableWeight => { pen.style_flags &= !(StyleFlags::Bold | StyleFlags::Dim); },
                GraphicStyle::DisableItalic => { pen.style_flags &= !StyleFlags::Italic; },
                GraphicStyle::DisableUnderline => { pen.style_flags &= !StyleFlags::Underline; },
                GraphicStyle::DisableBlinking => { pen.style_flags &= !StyleFlags::Blinking; },
                GraphicStyle::DisableInverse => { pen.style_flags &= !StyleFlags::Inverse; },
                GraphicStyle::DisableHidden => { pen.style_flags &= !StyleFlags::Hidden; },
                GraphicStyle::DisableStrikethrough => { pen.style_flags &= !StyleFlags::Strikethrough; },
                // foreground colours
                GraphicStyle::ForegroundBlack => { pen.foreground_colour = self.colour_table[0]; },
                GraphicStyle::ForegroundRed => { pen.foreground_colour = self.colour_table[1]; },
                GraphicStyle::ForegroundGreen => { pen.foreground_colour = self.colour_table[2]; },
                GraphicStyle::ForegroundYellow => { pen.foreground_colour = self.colour_table[3]; },
                GraphicStyle::ForegroundBlue => { pen.foreground_colour = self.colour_table[4]; },
                GraphicStyle::ForegroundMagenta => { pen.foreground_colour = self.colour_table[5]; },
                GraphicStyle::ForegroundCyan => { pen.foreground_colour = self.colour_table[6]; },
                GraphicStyle::ForegroundWhite => { pen.foreground_colour = self.colour_table[7]; },
                GraphicStyle::ForegroundExtended => { log::info!("[vt100] GraphicStyle({:?})", style); },
                GraphicStyle::ForegroundDefault => { pen.foreground_colour = self.default_pen.foreground_colour; },
                // background colours
                GraphicStyle::BackgroundBlack => { pen.background_colour = self.colour_table[0]; },
                GraphicStyle::BackgroundRed => { pen.background_colour = self.colour_table[1]; },
                GraphicStyle::BackgroundGreen => { pen.background_colour = self.colour_table[2]; },
                GraphicStyle::BackgroundYellow => { pen.background_colour = self.colour_table[3]; },
                GraphicStyle::BackgroundBlue => { pen.background_colour = self.colour_table[4]; },
                GraphicStyle::BackgroundMagenta => { pen.background_colour = self.colour_table[5]; },
                GraphicStyle::BackgroundCyan => { pen.background_colour = self.colour_table[6]; },
                GraphicStyle::BackgroundWhite => { pen.background_colour = self.colour_table[7]; },
                GraphicStyle::BackgroundExtended => { log::info!("[vt100] GraphicStyle({:?})", style); },
                GraphicStyle::BackgroundDefault => { pen.background_colour = self.default_pen.background_colour; },
                // bright foreground colours
                GraphicStyle::BrightForegroundBlack => { pen.foreground_colour = self.colour_table[0]; },
                GraphicStyle::BrightForegroundRed => { pen.foreground_colour = self.colour_table[1]; },
                GraphicStyle::BrightForegroundGreen => { pen.foreground_colour = self.colour_table[2]; },
                GraphicStyle::BrightForegroundYellow => { pen.foreground_colour = self.colour_table[3]; },
                GraphicStyle::BrightForegroundBlue => { pen.foreground_colour = self.colour_table[4]; },
                GraphicStyle::BrightForegroundMagenta => { pen.foreground_colour = self.colour_table[5]; },
                GraphicStyle::BrightForegroundCyan => { pen.foreground_colour = self.colour_table[6]; },
                GraphicStyle::BrightForegroundWhite => { pen.foreground_colour = self.colour_table[7]; },
                // bright background colours
                GraphicStyle::BrightBackgroundBlack => { pen.background_colour = self.colour_table[0]; },
                GraphicStyle::BrightBackgroundRed => { pen.background_colour = self.colour_table[1]; },
                GraphicStyle::BrightBackgroundGreen => { pen.background_colour = self.colour_table[2]; },
                GraphicStyle::BrightBackgroundYellow => { pen.background_colour = self.colour_table[3]; },
                GraphicStyle::BrightBackgroundBlue => { pen.background_colour = self.colour_table[4]; },
                GraphicStyle::BrightBackgroundMagenta => { pen.background_colour = self.colour_table[5]; },
                GraphicStyle::BrightBackgroundCyan => { pen.background_colour = self.colour_table[6]; },
                GraphicStyle::BrightBackgroundWhite => { pen.background_colour = self.colour_table[7]; },
            }
        }
    }
}

impl Handler for Terminal {
    fn on_ascii_data(&mut self, buf: &[u8]) {
        for b in buf {
            self.viewport.write_ascii(*b);
        }
    }

    fn on_utf8(&mut self, character: char) {
        self.viewport.write_cell(character);
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
                let pen = self.viewport.get_pen_mut();
                pen.background_colour = *rgb;
            },
            Vt100Command::SetForegroundColourRgb(ref rgb) => {
                let pen = self.viewport.get_pen_mut();
                pen.foreground_colour = *rgb;
            },
            Vt100Command::SetBackgroundColourTable(index) => {
                let pen = self.viewport.get_pen_mut();
                pen.background_colour = self.colour_table[*index as usize];
            },
            Vt100Command::SetForegroundColourTable(index) => {
                let pen = self.viewport.get_pen_mut();
                pen.foreground_colour = self.colour_table[*index as usize];
            },
            Vt100Command::MoveCursorPositionViewport(ref pos) => {
                // top left corner is (1,1)
                let pos = Vector2::new((pos.x.get()-1) as usize, (pos.y.get()-1) as usize);
                self.viewport.set_cursor(pos);
            },
            Vt100Command::EraseInDisplay(mode) => match mode {
                EraseMode::FromCursorToEnd => {
                    let size = self.viewport.get_size();
                    let cursor = self.viewport.get_cursor();
                    for y in (cursor.y+1)..size.y {
                        let (line, status) = self.viewport.get_row_mut(y);
                        line.fill(Cell::default());
                        *status = LineStatus::default();
                    }
                    let (line, status) = self.viewport.get_row_mut(cursor.y);
                    line[cursor.x..].fill(Cell::default());
                    status.is_linebreak = false;
                    status.length = cursor.x;
                },
                EraseMode::FromCursorToStart => {
                    let cursor = self.viewport.get_cursor();
                    for y in 0..cursor.y {
                        let (line, status) = self.viewport.get_row_mut(y);
                        line.fill(Cell::default());
                        *status = LineStatus::default();
                    }
                    let (line, _) = self.viewport.get_row_mut(cursor.y);
                    line[..=cursor.x].fill(Cell::default());
                },
                EraseMode::EntireDisplay => {
                    let size = self.viewport.get_size();
                    for y in 0..size.y {
                        let (line, status) = self.viewport.get_row_mut(y);
                        line.fill(Cell::default());
                        *status = LineStatus::default();
                    }
                },
                EraseMode::SavedLines => {
                    let size = self.viewport.get_size();
                    for y in 0..size.y {
                        let (line, status) = self.viewport.get_row_mut(y);
                        line.fill(Cell::default());
                        *status = LineStatus::default();
                    }
                },
            },
            Vt100Command::EraseInLine(mode) => match mode {
                EraseMode::FromCursorToEnd => {
                    let cursor = self.viewport.get_cursor();
                    let (line, status) = self.viewport.get_row_mut(cursor.y);
                    line[cursor.x..].fill(Cell::default());
                    status.length = cursor.x;
                    status.is_linebreak = false;
                },
                EraseMode::FromCursorToStart => {
                    let cursor = self.viewport.get_cursor();
                    let (line, _) = self.viewport.get_row_mut(cursor.y);
                    line[..=cursor.x].fill(Cell::default());
                },
                EraseMode::EntireDisplay => {
                    let cursor = self.viewport.get_cursor();
                    let (line, _) = self.viewport.get_row_mut(cursor.y);
                    line.fill(Cell::default());
                },
                EraseMode::SavedLines => {
                    let cursor = self.viewport.get_cursor();
                    let (line, _) = self.viewport.get_row_mut(cursor.y);
                    line.fill(Cell::default());
                },
            },
            Vt100Command::ReplaceWithSpaces(total) => {
                let size = self.viewport.get_size();
                let cursor = self.viewport.get_cursor();
                let (line, _) = self.viewport.get_row_mut(cursor.y);
                let end_index = (cursor.x+total.get() as usize).min(size.x);
                for c in &mut line[cursor.x..end_index] {
                    c.character = ' ';
                }
            },
            Vt100Command::MoveCursorRight(total) => {
                let mut cursor = self.viewport.get_cursor();
                cursor.x = cursor.x + total.get() as usize;
                self.viewport.set_cursor(cursor);
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
