use std::thread::JoinHandle;
use std::sync::{Arc, Mutex, MutexGuard};
use vt100::{
    command::Command as Vt100Command,
    encoder::{
        KeyCode,
        Encoder as Vt100Encoder,
    },
    parser::{
        Parser as Vt100Parser, 
        ParserError as Vt100ParserError,
    },
    common::{
        EraseMode,
        Rgb8,
        WindowAction,
    },
};
use crate::{
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
    KeyPress(KeyCode),
    KeyRelease(KeyCode),
    WindowResize(Vector2<usize>),
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
}

impl Terminal {
    pub fn new(mut builder: TerminalBuilder) -> Self {
        let display = Arc::new(Mutex::new(TerminalDisplay::default()));
        let encoder = Arc::new(Mutex::new(Vt100Encoder::default()));
        // parser thread 
        let mut parser_handler = ParserHandler {
            display: display.clone(),
            encoder: encoder.clone(),
            window_action: builder.window_action,
        };
        let parser_thread = std::thread::spawn(move || {
            let mut buffer = vec![0u8; 4096];
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
}

impl TerminalParserHandler for ParserHandler {
    fn on_ascii_data(&mut self, buf: &[u8]) {
        let mut display = self.display.lock().unwrap();
        for b in buf {
            display.write_ascii(*b);
        }
        let window_action = &mut self.window_action;
        window_action(WindowAction::Refresh);
    }

    fn on_utf8(&mut self, character: char) {
        let window_action = &mut self.window_action;
        let mut display = self.display.lock().unwrap();
        display.write_utf8(character);
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
                let mut display = self.display.lock().unwrap();
                display.set_graphic_style(style);
            },
            Vt100Command::SetBackgroundColourRgb(rgb) => {
                let mut display = self.display.lock().unwrap();
                let pen = display.get_pen_mut();
                // FIXME: Why is the background so bright???
                const A: u8 = 7;
                let rgb = Rgb8 { 
                    r: rgb.r / A,
                    g: rgb.g / A,
                    b: rgb.b / A,
                };
                pen.background_colour = rgb;
            },
            Vt100Command::SetForegroundColourRgb(rgb) => {
                let mut display = self.display.lock().unwrap();
                let pen = display.get_pen_mut();
                // FIXME: Why is the foreground so desaturated???
                const A: u8 = 20;
                let rgb = Rgb8 { 
                    r: rgb.r.saturating_sub(A),
                    g: rgb.g.saturating_sub(A),
                    b: rgb.b.saturating_sub(A),
                };
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
                    window_action(WindowAction::Refresh);
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
                    window_action(WindowAction::Refresh);
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
                    window_action(WindowAction::Refresh);
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
                    window_action(WindowAction::Refresh);
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
                    window_action(WindowAction::Refresh);
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
                    window_action(WindowAction::Refresh);
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
                window_action(WindowAction::Refresh);
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
                window_action(WindowAction::Refresh);
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
                window_action(WindowAction::Refresh);
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
                window_action(WindowAction::Refresh);
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
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorPositionViewport(pos) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                // top left corner is (1,1)
                let x = pos.x.saturating_sub(1) as usize;
                let y = pos.y.saturating_sub(1) as usize;
                viewport.set_cursor(Vector2::new(x,y));
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorUp(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.y = cursor.y.saturating_sub(total as usize);
                viewport.set_cursor(cursor);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorDown(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.y += total as usize;
                viewport.set_cursor(cursor);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorRight(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.x += total as usize;
                viewport.set_cursor(cursor);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorLeft(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.x = cursor.x.saturating_sub(total as usize);
                viewport.set_cursor(cursor);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorReverseIndex => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.y = cursor.y.saturating_sub(1);
                viewport.set_cursor(cursor);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorNextLine(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.y = total.saturating_sub(1) as usize;
                viewport.set_cursor(cursor);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorPreviousLine(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.y = total.saturating_sub(1) as usize;
                viewport.set_cursor(cursor);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorHorizontalAbsolute(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                let mut cursor = viewport.get_cursor();
                cursor.x = total.saturating_sub(1) as usize;
                viewport.set_cursor(cursor);
                window_action(WindowAction::Refresh);
            },
            Vt100Command::MoveCursorVerticalAbsolute(total) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
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
                display.save_cursor();
            },
            Vt100Command::RestoreCursorFromMemory => {
                let mut display = self.display.lock().unwrap();
                display.restore_cursor();
                window_action(WindowAction::Refresh);
            },
            // cursor status
            Vt100Command::SetCursorBlinking(is_blink) => {
                let mut display = self.display.lock().unwrap();
                let cursor = display.get_cursor_status_mut();
                cursor.is_blinking = is_blink;
                window_action(WindowAction::Refresh);
            },
            Vt100Command::SetCursorVisible(is_visible) => {
                let mut display = self.display.lock().unwrap();
                let cursor = display.get_cursor_status_mut();
                cursor.is_visible = is_visible;
                window_action(WindowAction::Refresh);
            },
            Vt100Command::SetCursorStyle(style) => {
                let mut display = self.display.lock().unwrap();
                let cursor = display.get_cursor_status_mut();
                cursor.style = style;
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
            // mouse (TODO)
            // Vt100Command::SetHighlightMouseTracking(is_tracking) => {
            //     let mut encoder = self.encoder.lock().unwrap();
            // },
            // Vt100Command::SetCellMouseTracking(is_tracking) => {
            //     let mut encoder = self.encoder.lock().unwrap();
            // },
            // Vt100Command::SetAllMouseTracking(is_tracking) => {
            //     let mut encoder = self.encoder.lock().unwrap();
            // },
            // window
            Vt100Command::WindowAction(action) => window_action(action),
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
            TerminalUserEvent::WindowResize(size) => {
                let mut display = self.display.lock().unwrap();
                let viewport = display.get_viewport_mut();
                viewport.set_size(size);
                process_ioctl(TerminalIOControl::SetSize(size));
                // Apparently this shouldnt be used when ioctl is available
                // let mut encoder = self.encoder.lock().unwrap();
                // encoder.set_window_size_characters(size, writer);
            },
            TerminalUserEvent::SetIsNewlineCarriageReturn(is_carriage_return) => {
                let mut display = self.display.lock().unwrap();
                display.set_is_newline_carriage_return(is_carriage_return);
            },
        }
    }
}
