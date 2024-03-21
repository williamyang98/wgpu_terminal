use std::sync::{Arc, Mutex};
use vt100::{
    command::Command as Vt100Command,
    parser::{
        Parser as Vt100Parser, 
        ParserError as Vt100ParserError,
    },
    common::{
        EraseMode,
        Rgb8,
    },
};
use crate::{
    terminal_parser::TerminalParserHandler,
    terminal_keyboard::TerminalKeyboard,
    terminal_display::TerminalDisplay,
    terminal_window::TerminalWindow,
    utf8_parser::ParserError as Utf8ParserError,
};
use cgmath::Vector2;

// converts parser output to thread safe commands to terminal components
pub struct TerminalCore {
    pub display: Arc<Mutex<TerminalDisplay>>,
    pub keyboard: Arc<Mutex<TerminalKeyboard>>,
    pub window: Box<dyn TerminalWindow + Send>,
}

impl TerminalParserHandler for TerminalCore {
    fn on_ascii_data(&mut self, buf: &[u8]) {
        let mut display = self.display.lock().unwrap();
        for b in buf {
            display.write_ascii(*b);
        }
    }

    fn on_utf8(&mut self, character: char) {
        let mut display = self.display.lock().unwrap();
        display.write_utf8(character);
    }

    fn on_unhandled_byte(&mut self, byte: u8) {
        log::error!("[unknown-byte] ({:?})", byte);
    }

    fn on_utf8_error(&mut self, error: &Utf8ParserError) {
        log::error!("[utf8-error] {:?}", error);
    }

    fn on_vt100(&mut self, c: Vt100Command) {
        match c {
            Vt100Command::SetHyperlink(link) => {
                log::info!("[vt100] SetHyperlink({})", link); 
            },
            // display
            Vt100Command::SetGraphicStyle(style) => {
                let mut display = self.display.lock().unwrap();
                display.set_graphic_style(style);
            },
            Vt100Command::SetBackgroundColourRgb(rgb) => {
                let mut display = self.display.lock().unwrap();
                let pen = display.get_pen_mut();
                // FIXME: Why is the background so bright???
                let rgb = Rgb8 { 
                    r: rgb.r / 6,
                    g: rgb.g / 6,
                    b: rgb.b / 6,
                };
                pen.background_colour = rgb;
            },
            Vt100Command::SetForegroundColourRgb(rgb) => {
                let mut display = self.display.lock().unwrap();
                let pen = display.get_pen_mut();
                pen.foreground_colour = rgb;
            },
            Vt100Command::SetBackgroundColourTable(index) => {
                let mut display = self.display.lock().unwrap();
                let colour = display.get_colour_from_table(index);
                let pen = display.get_pen_mut();
                pen.background_colour = colour;
            },
            Vt100Command::SetForegroundColourTable(index) => {
                let mut display = self.display.lock().unwrap();
                let colour = display.get_colour_from_table(index);
                let pen = display.get_pen_mut();
                pen.foreground_colour = colour;
            },
            // erase data
            Vt100Command::EraseInDisplay(mode) => match mode {
                EraseMode::FromCursorToEnd => {
                    let mut display = self.display.lock().unwrap();
                    let pen = *display.get_pen();
                    let viewport = display.get_viewport_mut();
                    let size = viewport.get_size();
                    let cursor = viewport.get_cursor();
                    for y in (cursor.y+1)..size.y {
                        let (line, status) = viewport.get_row_mut(y);
                        line.iter_mut().for_each(|c| {
                            c.character = ' '; 
                            pen.colour_in_cell(c);
                        });
                        status.length = size.x;
                        status.is_linebreak = true;
                    }
                    let (line, status) = viewport.get_row_mut(cursor.y);
                    line[cursor.x..status.length].iter_mut().for_each(|c| {
                        c.character = ' ';
                        pen.colour_in_cell(c);
                    });
                },
                EraseMode::FromCursorToStart => {
                    let mut display = self.display.lock().unwrap();
                    let pen = *display.get_pen();
                    let viewport = display.get_viewport_mut();
                    let size = viewport.get_size();
                    let cursor = viewport.get_cursor();
                    for y in 0..cursor.y {
                        let (line, status) = viewport.get_row_mut(y);
                        line.iter_mut().for_each(|c| {
                            c.character = ' '; 
                            pen.colour_in_cell(c);
                        });
                        status.length = size.x;
                        status.is_linebreak = true;
                    }
                    let (line, _) = viewport.get_row_mut(cursor.y);
                    line[..=cursor.x].iter_mut().for_each(|c| {
                        c.character = ' ';
                        pen.colour_in_cell(c);
                    });
                },
                EraseMode::EntireDisplay | EraseMode::SavedLines => {
                    let mut display = self.display.lock().unwrap();
                    let pen = *display.get_pen();
                    let viewport = display.get_viewport_mut();
                    let size = viewport.get_size();
                    for y in 0..size.y {
                        let (line, status) = viewport.get_row_mut(y);
                        line.iter_mut().for_each(|c| {
                            c.character = ' '; 
                            pen.colour_in_cell(c);
                        });
                        status.length = size.x;
                        status.is_linebreak = true;
                    }
                },
            },
            Vt100Command::EraseInLine(mode) => match mode {
                EraseMode::FromCursorToEnd => {
                    let mut display = self.display.lock().unwrap();
                    let pen = *display.get_pen();
                    let viewport = display.get_viewport_mut();
                    let size = viewport.get_size();
                    let cursor = viewport.get_cursor();
                    let (line, status) = viewport.get_row_mut(cursor.y);
                    line[cursor.x..].iter_mut().for_each(|c| {
                        c.character = ' '; 
                        pen.colour_in_cell(c);
                    });
                    status.length = size.x;
                    status.is_linebreak = true;
                },
                EraseMode::FromCursorToStart => {
                    let mut display = self.display.lock().unwrap();
                    let pen = *display.get_pen();
                    let viewport = display.get_viewport_mut();
                    let cursor = viewport.get_cursor();
                    let (line, _) = viewport.get_row_mut(cursor.y);
                    line[..=cursor.x].iter_mut().for_each(|c| {
                        c.character = ' '; 
                        pen.colour_in_cell(c);
                    });
                },
                EraseMode::EntireDisplay | EraseMode::SavedLines => {
                    let mut display = self.display.lock().unwrap();
                    let pen = *display.get_pen();
                    let viewport = display.get_viewport_mut();
                    let size = viewport.get_size();
                    let cursor = viewport.get_cursor();
                    let (line, status) = viewport.get_row_mut(cursor.y);
                    line.iter_mut().for_each(|c| {
                        c.character = ' ';
                        pen.colour_in_cell(c);
                    });
                    status.length = size.x;
                    status.is_linebreak = true;
                },
            },
            Vt100Command::ReplaceWithSpaces(total) => {
                let mut display = self.display.lock().unwrap();
                let pen = *display.get_pen();
                let viewport = display.get_viewport_mut();
                let cursor = viewport.get_cursor();
                let (line, _) = viewport.get_row_mut(cursor.y);
                let region = &mut line[cursor.x..];
                let total = (total as usize).min(region.len());
                region[..total].iter_mut().for_each(|c| {
                    c.character = ' ';
                    pen.colour_in_cell(c);
                });
            },
            Vt100Command::InsertSpaces(total) => {
                let mut display = self.display.lock().unwrap();
                let pen = *display.get_pen();
                let viewport = display.get_viewport_mut();
                let cursor = viewport.get_cursor();
                let (line, status) = viewport.get_row_mut(cursor.y);
                let region = &mut line[cursor.x..];
                let total = (total as usize).min(region.len());
                let shift = region.len()-total;
                region.copy_within(0..shift, total);
                region[..total].iter_mut().for_each(|c| {
                    c.character = ' ';
                    pen.colour_in_cell(c);
                });
                status.length = (status.length+total).min(line.len());
            },
            Vt100Command::DeleteCharacters(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let cursor = viewport.get_cursor();
                let (line, status) = viewport.get_row_mut(cursor.y);
                let region = &mut line[(cursor.x+1)..];
                let total = (total as usize).min(region.len());
                region.copy_within(total.., 0);
                status.length = status.length.saturating_sub(total);
            },
            Vt100Command::InsertLines(total_insert) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let cursor = viewport.get_cursor();
                let size = viewport.get_size();
                let lines_at_cursor = size.y-cursor.y;
                let total_insert = (total_insert as usize).min(lines_at_cursor);
                let total_copy = lines_at_cursor-total_insert;
                viewport.copy_lines_within(cursor.y, cursor.y+total_insert, total_copy);
                for i in 0..total_insert {
                    let (_, status) = viewport.get_row_mut(cursor.y+i);
                    status.length = 0;
                    status.is_linebreak = true;
                }
            },
            Vt100Command::DeleteLines(total_delete) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let cursor = viewport.get_cursor();
                let size = viewport.get_size();
                let lines_at_cursor = size.y-cursor.y;
                let total_delete = (total_delete as usize).min(lines_at_cursor);
                let total_copy = lines_at_cursor-total_delete;
                viewport.copy_lines_within(cursor.y+total_delete, cursor.y, total_copy);
                for i in 0..total_delete {
                    let (_, status) = viewport.get_row_mut(cursor.y+total_copy+i);
                    status.length = 0;
                    status.is_linebreak = true;
                }
            },
            Vt100Command::MoveCursorPositionViewport(pos) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                // top left corner is (1,1)
                let x = pos.x.saturating_sub(1) as usize;
                let y = pos.y.saturating_sub(1) as usize;
                viewport.set_cursor(Vector2::new(x,y));
            },
            Vt100Command::MoveCursorUp(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.y = cursor.y.saturating_sub(total as usize);
                viewport.set_cursor(cursor);
            },
            Vt100Command::MoveCursorDown(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.y += total as usize;
                viewport.set_cursor(cursor);
            },
            Vt100Command::MoveCursorRight(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.x += total as usize;
                viewport.set_cursor(cursor);
            },
            Vt100Command::MoveCursorLeft(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.x = cursor.x.saturating_sub(total as usize);
                viewport.set_cursor(cursor);
            },
            Vt100Command::MoveCursorReverseIndex => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.y = cursor.y.saturating_sub(1);
                viewport.set_cursor(cursor);
            },
            Vt100Command::MoveCursorNextLine(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.y = total.saturating_sub(1) as usize;
                viewport.set_cursor(cursor);
            },
            Vt100Command::MoveCursorPreviousLine(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.y = total.saturating_sub(1) as usize;
                viewport.set_cursor(cursor);
            },
            Vt100Command::MoveCursorHorizontalAbsolute(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.x = total.saturating_sub(1) as usize;
                viewport.set_cursor(cursor);
            },
            Vt100Command::MoveCursorVerticalAbsolute(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.y = total.saturating_sub(1) as usize;
                viewport.set_cursor(cursor);
            },
            Vt100Command::ScrollUp(_total) => {
                // TODO:
            },
            Vt100Command::ScrollDown(_total) => {
                // TODO:
            },
            Vt100Command::SaveCursorToMemory => {
                let mut display = self.display.lock().unwrap();
                display.save_cursor();
            },
            Vt100Command::RestoreCursorFromMemory => {
                let mut display = self.display.lock().unwrap();
                display.restore_cursor();
            },
            // cursor status
            Vt100Command::SetCursorBlinking(is_blink) => {
                let mut display = self.display.lock().unwrap();
                let cursor = display.get_cursor_status_mut();
                cursor.is_blinking = is_blink;
            },
            Vt100Command::SetCursorVisible(is_visible) => {
                let mut display = self.display.lock().unwrap();
                let cursor = display.get_cursor_status_mut();
                cursor.is_visible = is_visible;
            },
            Vt100Command::SetCursorStyle(style) => {
                let mut display = self.display.lock().unwrap();
                let cursor = display.get_cursor_status_mut();
                cursor.style = style;
            },
            // keyboard
            Vt100Command::SetKeypadMode(input_mode) => {
                let mut keyboard = self.keyboard.lock().unwrap();
                keyboard.set_keypad_input_mode(input_mode);
            },
            Vt100Command::SetCursorKeyInputMode(input_mode) => {
                let mut keyboard = self.keyboard.lock().unwrap();
                keyboard.set_cursor_key_input_mode(input_mode);
            },
            Vt100Command::SetBracketedPasteMode(is_bracketed) => {
                let mut keyboard = self.keyboard.lock().unwrap();
                keyboard.set_is_bracketed_paste_mode(is_bracketed);
            },
            // window
            Vt100Command::WindowAction(action) => {
                self.window.on_window_action(action);
            },
            _ => {
                log::info!("[vt100] Unhandled: {:?}", c);
            },
        }
    }

    fn on_vt100_error(&mut self, err: Vt100ParserError, parser: &Vt100Parser) {
        log::error!("[vt100-error] {:?} {:?}", err, parser);
    }

}
