use crate::utf8_parser::{
    Parser as Utf8Parser,
    ParserError as Utf8ParserError,
};
use vt100::parser::{
    Parser as Vt100Parser, 
    ParserHandler as Vt100ParserHandler,
    VT100_ESCAPE_CODE,
};

enum State {
    Byte,
    Utf8,
    Vt100,
}

pub struct TerminalParser {
    state: State,
    utf8_parser: Utf8Parser,
    vt100_parser: Vt100Parser,
}

impl Default for TerminalParser {
    fn default() -> Self {
        Self {
            state: State::Byte,
            utf8_parser: Utf8Parser::default(),
            vt100_parser: Vt100Parser::default(),
        }
    }
}

pub trait TerminalParserHandler {
    fn on_unhandled_byte(&mut self, byte: u8);
    fn on_ascii_data(&mut self, buf: &[u8]);
    fn on_utf8(&mut self, character: char);
    fn on_utf8_error(&mut self, error: &Utf8ParserError);
}

impl TerminalParser {
    pub fn parse_bytes<T>(&mut self, mut buf: &[u8], handler: &mut T) 
    where T: TerminalParserHandler + Vt100ParserHandler
    {
        while !buf.is_empty() {
            match self.state {
                State::Byte => {
                    let mut total_ascii = 0;
                    let mut total_read = 0;
                    for &b in buf {
                        total_read += 1;
                        if b == VT100_ESCAPE_CODE {
                            self.state = State::Vt100;
                            self.vt100_parser.reset();
                            break;
                        }
                        if b & 0b1000_0000 == 0b0000_0000 { // ascii
                            total_ascii += 1;
                            continue;
                        } 
                        if self.utf8_parser.parse_header_byte(b) {
                            self.state = State::Utf8;
                            break;
                        }
                        handler.on_unhandled_byte(b);
                        break;
                    }
                    let ascii_buf = &buf[..total_ascii];
                    buf = &buf[total_read..];
                    if !ascii_buf.is_empty() {
                        handler.on_ascii_data(ascii_buf);
                    }
                },
                State::Utf8 => {
                    let mut total_read = 0;
                    for &b in buf {
                        total_read += 1;
                        match self.utf8_parser.parse_body_byte(b) {
                            Err(Utf8ParserError::Pending) => {},
                            Err(ref err) => {
                                handler.on_utf8_error(err);
                                self.state = State::Byte;
                                break;
                            },
                            Ok(c) => {
                                handler.on_utf8(c);
                                self.state = State::Byte;
                                break;
                            },
                        }
                    }
                    buf = &buf[total_read..];
                },
                State::Vt100 => {
                    let mut total_read = 0;
                    for &b in buf {
                        total_read += 1;
                        self.vt100_parser.feed_byte(b, handler);
                        if self.vt100_parser.is_terminated() {
                            self.state = State::Byte;
                            break;
                        }
                    }
                    buf = &buf[total_read..];
                },
            }
        }
    }
}
