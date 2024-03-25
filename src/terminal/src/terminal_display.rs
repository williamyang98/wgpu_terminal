use crate::{
    primitives::{Cell,StyleFlags, Pen},
    colour_table::{XTERM_COLOUR_TABLE, convert_u32_to_rgb},
    viewport::Viewport,
};
use vt100::{
    common::{
        CursorStyle,
        GraphicStyle,
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

#[derive(Clone,Debug)]
pub struct TerminalDisplay {
    viewport: Viewport,
    saved_cursor: Option<Vector2<usize>>,
    colour_table: Vec<Rgb8>,
    pub(crate) cursor_status: CursorStatus,
    pub(crate) pen: Pen,
    pub(crate) default_pen: Pen,
    pub(crate) is_newline_carriage_return: bool, // if true then \n will also set cursor.x = 0
}

impl Default for TerminalDisplay {
    fn default() -> Self {
        let colour_table: Vec<Rgb8> = XTERM_COLOUR_TABLE
            .iter()
            .map(|v| {
                const A: u8 = 80;
                let mut rgb = convert_u32_to_rgb(*v);
                rgb.r = rgb.r.saturating_add(A);
                rgb.g = rgb.g.saturating_add(A);
                rgb.b = rgb.b.saturating_add(A);
                rgb
            })
            .collect();
        assert!(colour_table.len() == 256);
        let default_pen = Pen {
            foreground_colour: Rgb8 { r: 255, b: 255, g: 255 },
            background_colour: Rgb8 { r: 5, b: 10, g: 7 },
            style_flags: StyleFlags::None,
        };
        Self {
            viewport: Viewport::default(),
            cursor_status: CursorStatus::default(),
            saved_cursor: None,
            pen: default_pen,
            default_pen,
            colour_table,
            is_newline_carriage_return: false,
        }
    }
}

impl TerminalDisplay {
    pub fn get_viewport(&self) -> &Viewport {
        &self.viewport
    }

    pub fn get_viewport_mut(&mut self) -> &mut Viewport {
        &mut self.viewport
    }

    pub fn set_graphic_style(&mut self, style: GraphicStyle) {
        let pen = &mut self.pen;
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

    #[inline]
    pub fn write_utf8(&mut self, character: char) {
        let mut cell = Cell { character, ..Cell::default() };
        self.pen.colour_in_cell(&mut cell);
        self.viewport.write_cell(&cell);
    }

    #[inline]
    pub fn get_colour_from_table(&self, index: u8) -> Rgb8 {
        self.colour_table[index as usize]
    }
 
    #[inline]
    pub fn write_ascii(&mut self, b: u8) {
        match b {
            b'\n' => {
                self.viewport.feed_newline(true);
                if self.is_newline_carriage_return {
                    self.viewport.carriage_return();
                }
            }, 
            b'\r' => self.viewport.carriage_return(),
            b'\x08' => { 
                let mut cursor = self.viewport.get_cursor(); 
                cursor.x = cursor.x.saturating_sub(1);
                self.viewport.set_cursor(cursor);
            },
            b' '..=b'~' => { self.write_utf8(b as char); },
            b'\x07' => { log::info!("Ding ding ding (BELL)"); },
            b => { log::error!("Unhandled byte: {}", b); },
        }
    }

    pub fn set_is_newline_carriage_return(&mut self, v: bool) {
        self.is_newline_carriage_return = v;
    }

    pub(crate) fn save_cursor(&mut self) {
        let cursor = self.viewport.get_cursor();
        self.saved_cursor = Some(cursor);
    }

    pub(crate) fn restore_cursor(&mut self) {
        match self.saved_cursor {
            Some(cursor) => self.viewport.set_cursor(cursor),
            None => log::warn!("tried to restore nonexistent cursor from memory"),
        }
    }
}
