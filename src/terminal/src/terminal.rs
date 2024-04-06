#![allow(clippy::type_complexity)]
use std::thread::JoinHandle;
use std::sync::{Arc, Mutex, MutexGuard};
use vt100::{
    command::Command as Vt100Command,
    encoder::{
        Encoder as Vt100Encoder,
        KeyCode,
        MouseButton,
        MouseEvent,
        MouseTrackingMode,
    },
    parser::{
        Parser as Vt100Parser, 
        ParserError as Vt100ParserError,
    },
    common::{
        EraseMode,
        Rgb8,
        WindowAction,
        GraphicStyle,
    },
};
use crate::{
    primitives::{Pen, StyleFlags},
    colour_table::{XTERM_COLOUR_TABLE, convert_u32_to_rgb},
    terminal_parser::{TerminalParser, TerminalParserHandler},
    terminal_display::TerminalDisplay,
    utf8_parser::ParserError as Utf8ParserError,
};
use cgmath::Vector2;
use crossbeam_channel::{
    Sender,
    bounded as channel,
};

// Some operating systems set/get terminal parameters over a separate pipe instead of stdout/stdin
// On linux this is ioctl and windows this is conpty
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum TerminalIOControl {
    SetSize(Vector2<usize>),
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum TerminalUserEvent {
    MousePress(MouseButton),
    MouseRelease(MouseButton),
    MouseMove(Vector2<usize>),
    KeyPress(KeyCode),
    KeyRelease(KeyCode),
    WindowResize(Vector2<usize>),
    WindowFocus(bool),
    GridResize(Vector2<usize>),
    SetIsNewlineCarriageReturn(bool),
}

pub struct Terminal {
    parser_thread: Option<JoinHandle<()>>,
    user_thread: (Sender<TerminalUserEvent>, JoinHandle<()>),
    display: Arc<Mutex<TerminalDisplay>>,
}

pub struct TerminalBuilder {
    pub process_read: Box<dyn FnMut(&mut [u8]) -> usize + Send>,
    pub process_write: Box<dyn FnMut(&[u8]) + Send>,
    pub process_ioctl: Box<dyn FnMut(TerminalIOControl) + Send>,
    pub window_action: Box<dyn FnMut(WindowAction) + Send>,
    pub is_newline_carriage_return: bool,
}

impl Terminal {
    pub fn new(mut builder: TerminalBuilder) -> Self {
        let mut display = TerminalDisplay::default();
        display.set_is_newline_carriage_return(builder.is_newline_carriage_return);
        // default colour table
        let colour_table: Vec<Rgb8> = XTERM_COLOUR_TABLE
            .iter()
            .map(|c| convert_u32_to_rgb(*c))
            .collect();
        let is_dark_mode = true;
        let default_pen = if is_dark_mode {
            Pen {
                background_colour: colour_table[0],
                foreground_colour: colour_table[15],
                style_flags: StyleFlags::default(),
            }
        } else {
            Pen {
                background_colour: colour_table[15],
                foreground_colour: colour_table[0],
                style_flags: StyleFlags::default(),
            }
        };
        display.set_default_pen(default_pen);
        display.get_current_viewport_mut().pen = default_pen;
        assert!(colour_table.len() == 256);
        // parser thread 
        let display = Arc::new(Mutex::new(display));
        let encoder = Arc::new(Mutex::new(Vt100Encoder::default()));
        let mut parser_handler = ParserHandler {
            display: display.clone(),
            encoder: encoder.clone(),
            window_action: builder.window_action,
            colour_table,
        };
        let parser_thread = std::thread::spawn(move || {
            let mut buffer = vec![0u8; 8192];
            let mut terminal_parser = TerminalParser::default();
            loop {
                let total_read = (builder.process_read)(buffer.as_mut_slice());
                if total_read == 0 {
                    break;
                }
                let src_buf = &buffer[..total_read];
                terminal_parser.parse_bytes(src_buf, &mut parser_handler);
            }
        });
        // user events thread
        let (user_tx, user_rx) = channel::<TerminalUserEvent>(256);
        let mut terminal_user = TerminalUser {
            display: display.clone(),
            encoder: encoder.clone(),
            process_write: builder.process_write,
            process_ioctl: builder.process_ioctl,
            mouse_position: Vector2::new(0,0),
        };
        let user_thread = std::thread::spawn(move || {
            while let Ok(event) = user_rx.recv() {
                terminal_user.on_event(event);
            }
        });

        Self {
            parser_thread: Some(parser_thread),
            user_thread: (user_tx, user_thread),
            display,
        }
    }

    pub fn join_parser_thread(&mut self) {
        if let Some(thread) = self.parser_thread.take() {
            if let Err(err) = thread.join() {
                log::error!("Failed to join terminal read thread: {:?}", err);
            }
        }
    }

    pub fn get_user_event_handler(&self) -> Sender<TerminalUserEvent> {
        self.user_thread.0.clone()
    }

    pub fn get_display(&mut self) -> MutexGuard<'_, TerminalDisplay> {
        let display = self.display.lock().unwrap();
        display
    }
}

struct ParserHandler {
    display: Arc<Mutex<TerminalDisplay>>,
    encoder: Arc<Mutex<Vt100Encoder>>,
    window_action: Box<dyn FnMut(WindowAction) + Send>,
    colour_table: Vec<Rgb8>,
}

impl ParserHandler {
    fn set_graphic_style(&mut self, style: GraphicStyle) {
        let mut display = self.display.lock().unwrap();
        let viewport = display.get_current_viewport_mut();
        match style {
            GraphicStyle::ResetAll => { viewport.pen = viewport.default_pen; },
            // flags
            GraphicStyle::EnableBold => { viewport.pen.style_flags |= StyleFlags::Bold; },
            GraphicStyle::EnableDim => { viewport.pen.style_flags |= StyleFlags::Dim; },
            GraphicStyle::EnableItalic => { viewport.pen.style_flags |= StyleFlags::Italic; },
            GraphicStyle::EnableUnderline => { viewport.pen.style_flags |= StyleFlags::Underline; },
            GraphicStyle::EnableBlinking => { viewport.pen.style_flags |= StyleFlags::Blinking; },
            GraphicStyle::EnableInverse => { viewport.pen.style_flags |= StyleFlags::Inverse; },
            GraphicStyle::EnableHidden => { viewport.pen.style_flags |= StyleFlags::Hidden; },
            GraphicStyle::EnableStrikethrough => { viewport.pen.style_flags |= StyleFlags::Strikethrough; },
            GraphicStyle::DisableWeight => { viewport.pen.style_flags &= !(StyleFlags::Bold | StyleFlags::Dim); },
            GraphicStyle::DisableItalic => { viewport.pen.style_flags &= !StyleFlags::Italic; },
            GraphicStyle::DisableUnderline => { viewport.pen.style_flags &= !StyleFlags::Underline; },
            GraphicStyle::DisableBlinking => { viewport.pen.style_flags &= !StyleFlags::Blinking; },
            GraphicStyle::DisableInverse => { viewport.pen.style_flags &= !StyleFlags::Inverse; },
            GraphicStyle::DisableHidden => { viewport.pen.style_flags &= !StyleFlags::Hidden; },
            GraphicStyle::DisableStrikethrough => { viewport.pen.style_flags &= !StyleFlags::Strikethrough; },
            // foreground colours
            GraphicStyle::ForegroundBlack => { viewport.pen.foreground_colour = self.colour_table[0]; },
            GraphicStyle::ForegroundRed => { viewport.pen.foreground_colour = self.colour_table[1]; },
            GraphicStyle::ForegroundGreen => { viewport.pen.foreground_colour = self.colour_table[2]; },
            GraphicStyle::ForegroundYellow => { viewport.pen.foreground_colour = self.colour_table[3]; },
            GraphicStyle::ForegroundBlue => { viewport.pen.foreground_colour = self.colour_table[4]; },
            GraphicStyle::ForegroundMagenta => { viewport.pen.foreground_colour = self.colour_table[5]; },
            GraphicStyle::ForegroundCyan => { viewport.pen.foreground_colour = self.colour_table[6]; },
            GraphicStyle::ForegroundWhite => { viewport.pen.foreground_colour = self.colour_table[7]; },
            GraphicStyle::ForegroundExtended => { log::info!("[vt100] GraphicStyle({:?})", style); },
            GraphicStyle::ForegroundDefault => { viewport.pen.foreground_colour = viewport.default_pen.foreground_colour; },
            // background colours
            GraphicStyle::BackgroundBlack => { viewport.pen.background_colour = self.colour_table[0]; },
            GraphicStyle::BackgroundRed => { viewport.pen.background_colour = self.colour_table[1]; },
            GraphicStyle::BackgroundGreen => { viewport.pen.background_colour = self.colour_table[2]; },
            GraphicStyle::BackgroundYellow => { viewport.pen.background_colour = self.colour_table[3]; },
            GraphicStyle::BackgroundBlue => { viewport.pen.background_colour = self.colour_table[4]; },
            GraphicStyle::BackgroundMagenta => { viewport.pen.background_colour = self.colour_table[5]; },
            GraphicStyle::BackgroundCyan => { viewport.pen.background_colour = self.colour_table[6]; },
            GraphicStyle::BackgroundWhite => { viewport.pen.background_colour = self.colour_table[7]; },
            GraphicStyle::BackgroundExtended => { log::info!("[vt100] GraphicStyle({:?})", style); },
            GraphicStyle::BackgroundDefault => { viewport.pen.background_colour = viewport.default_pen.background_colour; },
            // bright foreground colours
            GraphicStyle::BrightForegroundBlack => { viewport.pen.foreground_colour = self.colour_table[8]; },
            GraphicStyle::BrightForegroundRed => { viewport.pen.foreground_colour = self.colour_table[9]; },
            GraphicStyle::BrightForegroundGreen => { viewport.pen.foreground_colour = self.colour_table[10]; },
            GraphicStyle::BrightForegroundYellow => { viewport.pen.foreground_colour = self.colour_table[11]; },
            GraphicStyle::BrightForegroundBlue => { viewport.pen.foreground_colour = self.colour_table[12]; },
            GraphicStyle::BrightForegroundMagenta => { viewport.pen.foreground_colour = self.colour_table[13]; },
            GraphicStyle::BrightForegroundCyan => { viewport.pen.foreground_colour = self.colour_table[14]; },
            GraphicStyle::BrightForegroundWhite => { viewport.pen.foreground_colour = self.colour_table[15]; },
            // bright background colours
            GraphicStyle::BrightBackgroundBlack => { viewport.pen.background_colour = self.colour_table[8]; },
            GraphicStyle::BrightBackgroundRed => { viewport.pen.background_colour = self.colour_table[9]; },
            GraphicStyle::BrightBackgroundGreen => { viewport.pen.background_colour = self.colour_table[10]; },
            GraphicStyle::BrightBackgroundYellow => { viewport.pen.background_colour = self.colour_table[11]; },
            GraphicStyle::BrightBackgroundBlue => { viewport.pen.background_colour = self.colour_table[12]; },
            GraphicStyle::BrightBackgroundMagenta => { viewport.pen.background_colour = self.colour_table[13]; },
            GraphicStyle::BrightBackgroundCyan => { viewport.pen.background_colour = self.colour_table[14]; },
            GraphicStyle::BrightBackgroundWhite => { viewport.pen.background_colour = self.colour_table[15]; },
        }
    }

}

impl TerminalParserHandler for ParserHandler {
    fn on_ascii_data(&mut self, buf: &[u8]) {
        let mut display = self.display.lock().unwrap();
        let viewport = display.get_current_viewport_mut();
        for b in buf {
            viewport.write_ascii(*b);
        }
        let window_action = &mut self.window_action;
        window_action(WindowAction::Refresh);
    }

    fn on_utf8(&mut self, character: char) {
        let window_action = &mut self.window_action;
        let mut display = self.display.lock().unwrap();
        let viewport = display.get_current_viewport_mut();
        viewport.write_utf8(character);
        window_action(WindowAction::Refresh);
    }

    fn on_unhandled_byte(&mut self, byte: u8) {
        log::error!("[unknown-byte] ({:?})", byte);
    }

    fn on_utf8_error(&mut self, error: &Utf8ParserError) {
        log::error!("[utf8-error] {:?}", error);
    }

    fn on_vt100(&mut self, c: Vt100Command) {
        let window_action = &mut self.window_action;
        match c {
            Vt100Command::SetHyperlink(link) => {
                log::info!("[vt100] SetHyperlink({})", link); 
            },
            // display
            Vt100Command::SetGraphicStyle(style) => {
                self.set_graphic_style(style);
            },
            Vt100Command::SetBackgroundColourRgb(rgb) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                viewport.pen.background_colour = rgb;
            },
            Vt100Command::SetForegroundColourRgb(rgb) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                viewport.pen.foreground_colour = rgb;
            },
            Vt100Command::SetBackgroundColourTable(index) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                let colour = self.colour_table[index as usize];
                viewport.pen.background_colour = colour;
            },
            Vt100Command::SetForegroundColourTable(index) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                let colour = self.colour_table[index as usize];
                viewport.pen.foreground_colour = colour;
            },
            // erase data
            Vt100Command::EraseInDisplay(mode) => match mode {
                EraseMode::FromCursorToEnd => {
                    let mut display = self.display.lock().unwrap();
                    let viewport = display.get_current_viewport_mut();
                    let pen = viewport.pen;
                    let size = viewport.get_size();
                    let cursor = viewport.get_cursor();
                    for y in (cursor.y+1)..size.y {
                        let (line, status) = viewport.get_row_mut(y);
                        line.iter_mut().for_each(|c| {
                            c.character = ' '; 
                            c.pen = pen;
                        });
                        status.length = size.x;
                        status.is_linebreak = true;
                    }
                    let (line, status) = viewport.get_row_mut(cursor.y);
                    line[cursor.x..status.length].iter_mut().for_each(|c| {
                        c.character = ' ';
                        c.pen = pen;
                    });
                    window_action(WindowAction::Refresh);
                },
                EraseMode::FromCursorToStart => {
                    let mut display = self.display.lock().unwrap();
                    let viewport = display.get_current_viewport_mut();
                    let pen = viewport.pen;
                    let size = viewport.get_size();
                    let cursor = viewport.get_cursor();
                    for y in 0..cursor.y {
                        let (line, status) = viewport.get_row_mut(y);
                        line.iter_mut().for_each(|c| {
                            c.character = ' '; 
                            c.pen = pen;
                        });
                        status.length = size.x;
                        status.is_linebreak = true;
                    }
                    let (line, _) = viewport.get_row_mut(cursor.y);
                    line[..=cursor.x].iter_mut().for_each(|c| {
                        c.character = ' ';
                        c.pen = pen;
                    });
                    window_action(WindowAction::Refresh);
                },
                EraseMode::EntireDisplay | EraseMode::SavedLines => {
                    let mut display = self.display.lock().unwrap();
                    let viewport = display.get_current_viewport_mut();
                    let pen = viewport.pen;
                    let size = viewport.get_size();
                    for y in 0..size.y {
                        let (line, status) = viewport.get_row_mut(y);
                        line.iter_mut().for_each(|c| {
                            c.character = ' '; 
                            c.pen = pen;
                        });
                        status.length = size.x;
                        status.is_linebreak = true;
                    }
                    window_action(WindowAction::Refresh);
                },
            },
            Vt100Command::EraseInLine(mode) => match mode {
                EraseMode::FromCursorToEnd => {
                    let mut display = self.display.lock().unwrap();
                    let viewport = display.get_current_viewport_mut();
                    let pen = viewport.pen;
                    let size = viewport.get_size();
                    let cursor = viewport.get_cursor();
                    let (line, status) = viewport.get_row_mut(cursor.y);
                    line[cursor.x..].iter_mut().for_each(|c| {
                        c.character = ' '; 
                        c.pen = pen;
                    });
                    status.length = size.x;
                    status.is_linebreak = true;
                    window_action(WindowAction::Refresh);
                },
                EraseMode::FromCursorToStart => {
                    let mut display = self.display.lock().unwrap();
                    let viewport = display.get_current_viewport_mut();
                    let pen = viewport.pen;
                    let cursor = viewport.get_cursor();
                    let (line, _) = viewport.get_row_mut(cursor.y);
                    line[..=cursor.x].iter_mut().for_each(|c| {
                        c.character = ' '; 
                        c.pen = pen;
                    });
                    window_action(WindowAction::Refresh);
                },
                EraseMode::EntireDisplay | EraseMode::SavedLines => {
                    let mut display = self.display.lock().unwrap();
                    let viewport = display.get_current_viewport_mut();
                    let pen = viewport.pen;
                    let size = viewport.get_size();
                    let cursor = viewport.get_cursor();
                    let (line, status) = viewport.get_row_mut(cursor.y);
                    line.iter_mut().for_each(|c| {
                        c.character = ' ';
                        c.pen = pen;
                    });
                    status.length = size.x;
                    status.is_linebreak = true;
                    window_action(WindowAction::Refresh);
                },
            },
            Vt100Command::ReplaceWithSpaces(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                let pen = viewport.pen;
                let cursor = viewport.get_cursor();
                let (line, _) = viewport.get_row_mut(cursor.y);
                let region = &mut line[cursor.x..];
                let total = (total as usize).min(region.len());
                region[..total].iter_mut().for_each(|c| {
                    c.character = ' ';
                    c.pen = pen;
                });
                window_action(WindowAction::Refresh);
            },
            Vt100Command::InsertSpaces(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                let pen = viewport.pen;
                let cursor = viewport.get_cursor();
                let (line, status) = viewport.get_row_mut(cursor.y);
                let region = &mut line[cursor.x..];
                let total = (total as usize).min(region.len());
                let shift = region.len()-total;
                region.copy_within(0..shift, total);
                region[..total].iter_mut().for_each(|c| {
                    c.character = ' ';
                    c.pen = pen;
                });
                status.length = (status.length+total).min(line.len());
                window_action(WindowAction::Refresh);
            },
            Vt100Command::DeleteCharacters(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                let cursor = viewport.get_cursor();
                let (line, status) = viewport.get_row_mut(cursor.y);
                let region = &mut line[(cursor.x+1)..];
                let total = (total as usize).min(region.len());
                region.copy_within(total.., 0);
                status.length = status.length.saturating_sub(total);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::InsertLines(total_insert) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
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
                window_action(WindowAction::Refresh);
            },
            Vt100Command::DeleteLines(total_delete) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
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
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorPositionViewport(pos) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                // top left corner is (1,1)
                let x = pos.x.saturating_sub(1) as usize;
                let y = pos.y.saturating_sub(1) as usize;
                viewport.set_cursor(Vector2::new(x,y));
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorUp(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.y = cursor.y.saturating_sub(total as usize);
                viewport.set_cursor(cursor);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorDown(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.y += total as usize;
                viewport.set_cursor(cursor);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorRight(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.x += total as usize;
                viewport.set_cursor(cursor);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorLeft(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.x = cursor.x.saturating_sub(total as usize);
                viewport.set_cursor(cursor);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorReverseIndex => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.y = cursor.y.saturating_sub(1);
                viewport.set_cursor(cursor);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorNextLine(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.y = total.saturating_sub(1) as usize;
                viewport.set_cursor(cursor);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorPreviousLine(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.y = total.saturating_sub(1) as usize;
                viewport.set_cursor(cursor);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorHorizontalAbsolute(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.x = total.saturating_sub(1) as usize;
                viewport.set_cursor(cursor);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorVerticalAbsolute(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.y = total.saturating_sub(1) as usize;
                viewport.set_cursor(cursor);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::ScrollUp(_total) => {
                // TODO:
            },
            Vt100Command::ScrollDown(_total) => {
                // TODO:
            },
            Vt100Command::SaveCursorToMemory => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                viewport.save_cursor();
            },
            Vt100Command::RestoreCursorFromMemory => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_current_viewport_mut();
                viewport.restore_cursor();
                window_action(WindowAction::Refresh);
            },
            // cursor status
            Vt100Command::SetCursorBlinking(is_blink) => {
                let mut display = self.display.lock().unwrap();
                display.cursor_status.is_blinking = is_blink;
                window_action(WindowAction::Refresh);
            },
            Vt100Command::SetCursorVisible(is_visible) => {
                let mut display = self.display.lock().unwrap();
                display.cursor_status.is_visible = is_visible;
                window_action(WindowAction::Refresh);
            },
            Vt100Command::SetCursorStyle(style) => {
                let mut display = self.display.lock().unwrap();
                display.cursor_status.style = style;
                window_action(WindowAction::Refresh);
            },
            // keyboard
            Vt100Command::SetKeypadMode(input_mode) => {
                let mut encoder = self.encoder.lock().unwrap();
                encoder.keypad_input_mode = input_mode;
            },
            Vt100Command::SetCursorKeyInputMode(input_mode) => {
                let mut encoder = self.encoder.lock().unwrap();
                encoder.cursor_key_input_mode = input_mode;
            },
            Vt100Command::SetBracketedPasteMode(is_bracketed) => {
                let mut encoder = self.encoder.lock().unwrap();
                encoder.is_bracketed_paste_mode = is_bracketed;
            },
            // mouse
            Vt100Command::SetMouseTrackingMode(mut mode) => {
                if mode == MouseTrackingMode::Highlight {
                    mode = MouseTrackingMode::Normal;
                }
                let mut encoder = self.encoder.lock().unwrap();
                encoder.mouse_tracking_mode = mode;
            },
            Vt100Command::SetMouseCoordinateFormat(format) => {
                let mut encoder = self.encoder.lock().unwrap();
                encoder.mouse_coordinate_format = format;
            },
            Vt100Command::SetReportFocus(is_report_focus) => {
                let mut encoder = self.encoder.lock().unwrap();
                encoder.is_report_focus = is_report_focus;
            },
            // window
            Vt100Command::WindowAction(action) => window_action(action),
            // alternate buffer
            Vt100Command::SetAlternateBuffer(is_alternate) => {
                let mut display = self.display.lock().unwrap();
                display.set_is_alternate(is_alternate);
            }
            _ => {
                log::info!("[vt100] Unhandled: {:?}", c);
            },
        }
    }

    fn on_vt100_error(&mut self, err: Vt100ParserError, parser: &Vt100Parser) {
        log::error!("[vt100-error] {:?} {:?}", err, parser);
    }
}

// Terminal user
struct TerminalUser {
    display: Arc<Mutex<TerminalDisplay>>,
    encoder: Arc<Mutex<Vt100Encoder>>,
    process_write: Box<dyn FnMut(&[u8]) + Send>,
    process_ioctl: Box<dyn FnMut(TerminalIOControl) + Send>,
    mouse_position: Vector2<usize>,
}

impl TerminalUser {
    fn on_event(&mut self, event: TerminalUserEvent) {
        let process_write = &mut self.process_write;
        let process_ioctl = &mut self.process_ioctl;

        match event {
            TerminalUserEvent::KeyPress(key_code) => {
                let mut encoder = self.encoder.lock().unwrap();
                encoder.on_key_press(key_code, process_write);
            },
            TerminalUserEvent::KeyRelease(key_code) => {
                let mut encoder = self.encoder.lock().unwrap();
                encoder.on_key_release(key_code, process_write);
            },
            TerminalUserEvent::GridResize(size) => {
                let size = Vector2::new(size.x.max(1), size.y.max(1));
                let mut display = self.display.lock().unwrap();
                display.set_size(size);
                process_ioctl(TerminalIOControl::SetSize(size));
                // Apparently this shouldnt be used when ioctl is available
                // let mut encoder = self.encoder.lock().unwrap();
                // encoder.set_window_size_characters(size, writer);
                let mut encoder = self.encoder.lock().unwrap();
                encoder.grid_size = size;
            },
            TerminalUserEvent::WindowResize(size) => {
                let size = Vector2::new(size.x.max(1), size.y.max(1));
                let mut encoder = self.encoder.lock().unwrap();
                encoder.window_size = size;
            },
            TerminalUserEvent::SetIsNewlineCarriageReturn(is_carriage_return) => {
                let mut display = self.display.lock().unwrap();
                display.set_is_newline_carriage_return(is_carriage_return);
            },
            TerminalUserEvent::MouseMove(pos) => {
                self.mouse_position = pos;
                let mut encoder = self.encoder.lock().unwrap();
                encoder.on_mouse_event(MouseEvent::Move(self.mouse_position), process_write);
            },
            TerminalUserEvent::MousePress(button) => {
                let mut encoder = self.encoder.lock().unwrap();
                encoder.on_mouse_event(MouseEvent::ButtonPress(button, self.mouse_position), process_write);
            },
            TerminalUserEvent::MouseRelease(button) => {
                let mut encoder = self.encoder.lock().unwrap();
                encoder.on_mouse_event(MouseEvent::ButtonRelease(button, self.mouse_position), process_write);
            },
            TerminalUserEvent::WindowFocus(is_focus) => {
                let encoder = self.encoder.lock().unwrap();
                encoder.on_window_focus(is_focus, process_write);
            },
        }
    }
}
