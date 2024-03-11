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

#[derive(Clone,Copy,Debug)]
pub struct CursorStatus {
    is_visible: bool,
    is_blinking: bool,
}

impl Default for CursorStatus {
    fn default() -> Self {
        Self {
            is_visible: true,
            is_blinking: true,
        }
    }
}


pub struct Terminal {
    viewport: Viewport,
    cursor_status: CursorStatus,
    saved_cursor: Option<Vector2<usize>>,
    default_pen: Pen,
    colour_table: Vec<Rgb8>,
}

impl Default for Terminal {
    fn default() -> Self {
        let colour_table: Vec<Rgb8> = XTERM_COLOUR_TABLE
            .iter()
            .map(|v| convert_u32_to_rgb(*v))
            .collect();
        assert!(colour_table.len() == 256);
        let default_pen = Pen {
            foreground_colour: Rgb8 { r: 255, b: 255, g: 255 },
            background_colour: Rgb8 { r: 0, b: 0, g: 0 },
            style_flags: StyleFlags::None,
        };
        let mut res = Self {
            viewport: Viewport::default(),
            cursor_status: CursorStatus::default(),
            saved_cursor: None,
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
        self.viewport.write_utf8(character);
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
            // set pen colour
            Vt100Command::SetGraphicStyles(ref styles) => {
                self.set_graphic_styles(styles);
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
            // erase data
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
            Vt100Command::InsertSpaces(total) => {
                let cursor = self.viewport.get_cursor();
                let (line, _) = self.viewport.get_row_mut(cursor.y);
                let line = &mut line[cursor.x..];
                let total = total.get() as usize;
                let width = line.len();
                let total = total.min(width);
                let shift = width-total;
                for i in (0..shift).rev() {
                    let dst_i = i+total;
                    let src_i = i;
                    line[dst_i] = line[src_i];
                }
                for i in 0..total {
                    line[i] = Cell::default();
                }
            },
            Vt100Command::DeleteCharacters(total) => {
                let cursor = self.viewport.get_cursor();
                let (line, _) = self.viewport.get_row_mut(cursor.y);
                let line = &mut line[cursor.x..];
                let total = total.get() as usize;
                let width = line.len();
                let total = total.min(width);
                let shift = width-total;
                for i in 0..shift {
                    let dst_i = i;
                    let src_i = i+total;
                    line[dst_i] = line[src_i];
                }
                for i in 0..total {
                    line[i+shift] = Cell::default();
                }
            },
            Vt100Command::InsertLines(total) => {
                self.viewport.insert_lines(total.get() as usize); 
            },
            Vt100Command::DeleteLines(total) => {
                self.viewport.delete_lines(total.get() as usize); 
            },
            // cursor movement
            Vt100Command::MoveCursorPositionViewport(ref pos) => {
                // top left corner is (1,1)
                let pos = Vector2::new((pos.x.get()-1) as usize, (pos.y.get()-1) as usize);
                self.viewport.set_cursor(pos);
            },
            Vt100Command::MoveCursorUp(total) => {
                let mut cursor = self.viewport.get_cursor();
                let total = total.get() as usize;
                cursor.y = cursor.y.max(total) - total;
                self.viewport.set_cursor(cursor);
            },
            Vt100Command::MoveCursorDown(total) => {
                let mut cursor = self.viewport.get_cursor();
                cursor.y += total.get() as usize;
                self.viewport.set_cursor(cursor);
            },
            Vt100Command::MoveCursorRight(total) => {
                let mut cursor = self.viewport.get_cursor();
                cursor.x += total.get() as usize;
                self.viewport.set_cursor(cursor);
            },
            Vt100Command::MoveCursorLeft(total) => {
                let mut cursor = self.viewport.get_cursor();
                let total = total.get() as usize;
                cursor.x = cursor.x.max(total) - total;
                self.viewport.set_cursor(cursor);
            },
            Vt100Command::MoveCursorReverseIndex => {
                let mut cursor = self.viewport.get_cursor();
                cursor.y = cursor.y.max(1) - 1;
                self.viewport.set_cursor(cursor);
            },
            Vt100Command::MoveCursorNextLine(total) => {
                let mut cursor = self.viewport.get_cursor();
                cursor.y = total.get() as usize - 1;
                self.viewport.set_cursor(cursor);
            },
            Vt100Command::MoveCursorPreviousLine(total) => {
                let mut cursor = self.viewport.get_cursor();
                cursor.y = total.get() as usize - 1;
                self.viewport.set_cursor(cursor);
            },
            Vt100Command::MoveCursorHorizontalAbsolute(total) => {
                let mut cursor = self.viewport.get_cursor();
                cursor.x = total.get() as usize - 1;
                self.viewport.set_cursor(cursor);
            },
            Vt100Command::MoveCursorVerticalAbsolute(total) => {
                let mut cursor = self.viewport.get_cursor();
                cursor.y = total.get() as usize - 1;
                self.viewport.set_cursor(cursor);
            },
            // viewport positioning
            Vt100Command::ScrollUp(total) => {
                for _ in 0..total.get() {
                    self.viewport.scroll_up();
                }
            },
            Vt100Command::ScrollDown(total) => {
                for _ in 0..total.get() {
                    self.viewport.scroll_down();
                }
            },
            // cursor save/load
            Vt100Command::SaveCursorToMemory => {
                let cursor = self.viewport.get_cursor();
                self.saved_cursor = Some(cursor);
            },
            Vt100Command::RestoreCursorFromMemory => {
                match self.saved_cursor {
                    Some(cursor) => self.viewport.set_cursor(cursor),
                    None => log::warn!("[vt100] tried to restore nonexistent cursor from memory"),
                }
            },
            // cursor status
            Vt100Command::EnableCursorBlinking => {
                self.cursor_status.is_blinking = true;
            },
            Vt100Command::DisableCursorBlinking => {
                self.cursor_status.is_blinking = false;
            },
            Vt100Command::HideCursor => {
                self.cursor_status.is_visible = false;
            },
            Vt100Command::ShowCursor => {
                self.cursor_status.is_visible = true;
            },
            _ => {
                log::info!("[vt100] Unhandled: {:?}", c);
            },
        }
    }

    fn on_vt100_error(&mut self, err: &Vt100ParserError, parser: &Vt100Parser) {
        log::error!("[vt100-error] {:?} {:?}", err, parser);
    }
}
