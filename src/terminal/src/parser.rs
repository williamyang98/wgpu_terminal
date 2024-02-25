use crate::utf8_parser::{
    Parser as Utf8Parser,
    ParserError as Utf8ParserError,
};
use vt100::parser::{
    Parser as Vt100Parser, 
    ParserError as Vt100ParserError,
    VT100_ESCAPE_CODE,
};
use vt100::command::Command as Vt100Command;

enum State {
    Byte,
    Utf8,
    Vt100,
}

pub struct Parser {
    state: State,
    utf8_parser: Utf8Parser,
    vt100_parser: Vt100Parser,
}

impl Default for Parser {
    fn default() -> Self {
        Self {
            state: State::Byte,
            utf8_parser: Utf8Parser::default(),
            vt100_parser: Vt100Parser::default(),
        }
    }
}

pub trait Handler {
    fn on_unhandled_byte(&mut self, byte: u8);
    fn on_ascii_data(&mut self, buf: &[u8]);
    fn on_utf8(&mut self, character: char);
    fn on_utf8_error(&mut self, error: &Utf8ParserError);
    fn on_vt100(&mut self, character: &Vt100Command);
    fn on_vt100_error(&mut self, error: &Vt100ParserError, parser: &Vt100Parser);
}

impl Parser {
    pub fn parse_bytes(&mut self, mut buf: &[u8], handler: &mut impl Handler) {
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
                        match self.vt100_parser.feed_byte(b) {
                            Err(Vt100ParserError::Pending) => {},
                            Err(ref err) => {
                                handler.on_vt100_error(err, &self.vt100_parser);
                                self.state = State::Byte;
                                break;
                            },
                            Ok(ref command) => {
                                handler.on_vt100(command);
                                self.state = State::Byte;
                                break;
                            },
                        }
                    }
                    buf = &buf[total_read..];
                },
            }
        }
    }
}
