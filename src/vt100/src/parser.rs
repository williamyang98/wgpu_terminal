// Sources: https://github.com/0x5c/VT100-Examples/blob/master/vt_seq.md
//          https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797
use std::num::NonZeroU16;
use std::fmt;
use crate::{
    command::Command,
    graphic_style::{GraphicStyle,Rgb8},
    misc::{EraseMode,Vector2,ScrollRegion,CharacterSet,InputMode},
    screen_mode::{ScreenMode},
};

pub const VT100_ESCAPE_CODE: u8 = 0x1B;

#[derive(Clone,Copy,Debug,PartialEq)]
pub enum ParserError {
    Pending,
    Unhandled,
    MissingNumbers { given: usize, expected: usize }, // given, required
    InvalidEraseMode(u16),
    InvalidScreenMode(u16),
    NoValidGraphicStyles,
}

#[derive(Clone,Copy,Debug,Default,PartialEq)]
enum ParserContext {
    #[default]
    EntryPoint,                         // ESC
    ControlSequenceIntroducer,          // ESC [
    ControlSequenceIntroducerNumbers,   // ESC [ <n>
    CommonPrivateMode,                  // ESC [ ?
    Exclamation,                        // ESC [ !
    ScreenMode,                         // ESC [ =
    Designate,                          // ESC (
    OperatingSystemCommand,             // ESC ]
}

#[derive(Clone,Copy,Debug,Default,PartialEq)]
enum ParserState {
    #[default]
    Characters,
    Numbers,
}

#[derive(Clone,Copy,Default,Debug,PartialEq)]
struct NumberSlice {
    start_index: usize,
    end_index: usize,
}

impl NumberSlice {
    fn new(index: usize) -> Self {
        Self {
            start_index: index,
            end_index: index,
        } 
    }
}

#[derive(Clone,Copy,Default,Debug,PartialEq)]
enum OperatingSystemCommandTerminator {
    #[default]
    Bell,      // 0x07 (Bell)
    Backslash, // ESC \\
}

pub struct Parser {
    state: ParserState,
    context: ParserContext,
    buffer: Vec<u8>,
    numbers: Vec<u16>,
    numbers_last_index: Option<usize>,
    number_slice: Option<NumberSlice>,
    graphic_styles: Vec<GraphicStyle>,
    osc_terminator: OperatingSystemCommandTerminator,
}

impl Default for Parser {
    fn default() -> Self {
        const MAX_EXPECTED_SEQUENCE: usize = 128;
        const MAX_EXPECTED_NUMBERS: usize = 16;
        Self { 
            state: ParserState::default(),
            context: ParserContext::default(),
            buffer: Vec::with_capacity(MAX_EXPECTED_SEQUENCE),
            numbers: Vec::with_capacity(MAX_EXPECTED_NUMBERS),
            numbers_last_index: None,
            number_slice: None,
            graphic_styles: Vec::with_capacity(MAX_EXPECTED_NUMBERS),
            osc_terminator: OperatingSystemCommandTerminator::default(),
        }
    }
}

impl Parser {
    pub fn reset(&mut self) {
        self.state = ParserState::default();
        self.context = ParserContext::default();
        self.buffer.clear();
        self.numbers.clear();
        self.numbers_last_index  = None;
        self.number_slice = None;
        self.graphic_styles.clear();
        self.osc_terminator = OperatingSystemCommandTerminator::default();
    }

    pub fn feed_byte(&mut self, b: u8) -> Result<Command, ParserError> {
        self.buffer.push(b);
        self.parse_byte(b)
    }

    fn parse_byte(&mut self, b: u8) -> Result<Command, ParserError> {
        match self.state {
            ParserState::Characters => {
                match self.context {
                    ParserContext::EntryPoint => self.read_entry_point(b),
                    ParserContext::ControlSequenceIntroducer => self.read_control_sequence_introducer(b),
                    ParserContext::ControlSequenceIntroducerNumbers => self.read_control_sequence_introducer_numbers(b),
                    ParserContext::CommonPrivateMode => self.read_common_private_mode(b),
                    ParserContext::Exclamation => self.read_exclamation(b),
                    ParserContext::ScreenMode => self.read_screen_mode(b),
                    ParserContext::Designate => self.read_designate(b),
                    ParserContext::OperatingSystemCommand => self.read_operating_system_command(b),
                }
            },
            ParserState::Numbers => self.read_numbers(b),
        }
    }

    fn read_entry_point(&mut self, b: u8) -> Result<Command, ParserError> {
        // @mark: ESC
        let n_default = NonZeroU16::new(1).unwrap();
        match b {
            b'A' => Ok(Command::MoveCursorUp(n_default)),
            b'B' => Ok(Command::MoveCursorDown(n_default)),
            b'C' => Ok(Command::MoveCursorRight(n_default)),
            b'D' => Ok(Command::MoveCursorLeft(n_default)),
            b'E' => Ok(Command::MoveCursorReverseIndex),
            b'7' => Ok(Command::SaveCursorToMemory),
            b'8' => Ok(Command::RestoreCursorFromMemory),
            b'=' => Ok(Command::SetKeypadMode(InputMode::Application)),
            b'>' => Ok(Command::SetKeypadMode(InputMode::Numeric)),
            b'H' => Ok(Command::SetTabStopAtCurrentColumn),
            b'M' => Ok(Command::MoveCursorUp(n_default)),
            b'[' => {
                self.context = ParserContext::ControlSequenceIntroducer;
                Err(ParserError::Pending)
            },
            b'(' => {
                self.context = ParserContext::Designate;
                Err(ParserError::Pending)
            },
            b']' => {
                self.context = ParserContext::OperatingSystemCommand;
                self.state = ParserState::Numbers; 
                Err(ParserError::Pending)
            },
            _ => Err(ParserError::Unhandled),
        }
    }

    fn read_control_sequence_introducer(&mut self, b: u8) -> Result<Command, ParserError> {
        // @mark: ESC [
        match b {
            b's' => Ok(Command::SaveCursorToMemory),
            b'u' => Ok(Command::RestoreCursorFromMemory),
            b'?' => {
                self.context = ParserContext::CommonPrivateMode;
                self.state = ParserState::Numbers; 
                Err(ParserError::Pending)
            },
            b'!' => {
                self.context = ParserContext::Exclamation;
                Err(ParserError::Pending)
            },
            b'=' => {
                self.context = ParserContext::ScreenMode;
                self.state = ParserState::Numbers;
                Err(ParserError::Pending)
            },
            _ => {
                self.context = ParserContext::ControlSequenceIntroducerNumbers;
                self.state = ParserState::Numbers; 
                self.parse_byte(b) // forward this byte to csi numbers branch
            },
        }
    }

    fn read_control_sequence_introducer_numbers(&mut self, b: u8) -> Result<Command, ParserError> {
        // @mark: ESC [ <n>
        match b {
            b'A' => Ok(Command::MoveCursorUp(self.read_optional_nonzero_u16())),
            b'B' => Ok(Command::MoveCursorDown(self.read_optional_nonzero_u16())),
            b'C' => Ok(Command::MoveCursorRight(self.read_optional_nonzero_u16())),
            b'D' => Ok(Command::MoveCursorLeft(self.read_optional_nonzero_u16())),
            b'E' => Ok(Command::MoveCursorNextLine(self.read_optional_nonzero_u16())),
            b'F' => Ok(Command::MoveCursorPreviousLine(self.read_optional_nonzero_u16())),
            b'G' => Ok(Command::MoveCursorHorizontalAbsolute(self.read_optional_nonzero_u16())),
            b'd' => Ok(Command::MoveCursorVerticalAbsolute(self.read_optional_nonzero_u16())),
            b'S' => Ok(Command::ScrollUp(self.read_optional_nonzero_u16())),
            b'T' => Ok(Command::ScrollDown(self.read_optional_nonzero_u16())),
            b'@' => Ok(Command::InsertSpaces(self.read_optional_nonzero_u16())),
            b'P' => Ok(Command::DeleteCharacters(self.read_optional_nonzero_u16())),
            b'X' => Ok(Command::ReplaceWithSpaces(self.read_optional_nonzero_u16())),
            b'L' => Ok(Command::InsertLines(self.read_optional_nonzero_u16())),
            b'M' => Ok(Command::DeleteLines(self.read_optional_nonzero_u16())),
            b'J' => Ok(Command::EraseInDisplay(self.try_read_erase_mode()?)),
            b'K' => Ok(Command::EraseInLine(self.try_read_erase_mode()?)),
            b'H' => match self.try_read_xy() {
                Ok(pos) => Ok(Command::MoveCursorPositionViewport(pos)),
                Err(_) => {
                    let pos = NonZeroU16::new(1).unwrap();
                    let pos = Vector2::new(pos,pos);
                    Ok(Command::MoveCursorPositionViewport(pos))
                },
            },
            b'f' => Ok(Command::MoveCursorPositionViewport(self.try_read_xy()?)),
            b'm' => Ok(self.try_read_graphics_command()?),
            b'r' => Ok(Command::SetScrollRegion(self.read_scrolling_region())),
            b'I' => Ok(Command::AdvanceCursorToTabStop(self.read_optional_nonzero_u16())),
            b'Z' => Ok(Command::ReverseCursorToTabStop(self.read_optional_nonzero_u16())),
            b'g' => match self.try_get_numbers(1)?.first().unwrap() {
                0 => Ok(Command::ClearCurrentTabStop),
                3 => Ok(Command::ClearAllTabStops),
                _ => Err(ParserError::Unhandled),
            },
            b'n' => match self.try_get_numbers(1)?.first().unwrap() {
                6 => Ok(Command::QueryCursorPosition),
                _ => Err(ParserError::Unhandled),
            },
            b'c' => match self.try_get_numbers(1)?.first().unwrap() {
                0 => Ok(Command::QueryTerminalIdentity),
                _ => Err(ParserError::Unhandled),
            },
            _ => Err(ParserError::Unhandled)
        }
    }

    fn read_common_private_mode(&self, b: u8) -> Result<Command, ParserError> {
        // @mark: ESC [ ? <n>
        let n = self.try_get_numbers(1)?;
        let n = n[0];
        match (n, b) {
            (   1, b'h') => Ok(Command::SetCursorKeysMode(InputMode::Application)),
            (   1, b'l') => Ok(Command::SetCursorKeysMode(InputMode::Numeric)),
            (   3, b'h') => Ok(Command::SetConsoleWidth(NonZeroU16::new(132).unwrap())),
            (   3, b'l') => Ok(Command::SetConsoleWidth(NonZeroU16::new(80).unwrap())),
            (  12, b'h') => Ok(Command::SetCursorBlinking(true)),
            (  12, b'l') => Ok(Command::SetCursorBlinking(false)),
            (  25, b'h') => Ok(Command::SetCursorVisible(true)),
            (  25, b'l') => Ok(Command::SetCursorVisible(false)),
            (  47, b'h') => Ok(Command::SaveScreen),
            (  47, b'l') => Ok(Command::RestoreScreen),
            (1049, b'h') => Ok(Command::SetAlternateBuffer(true)),
            (1049, b'l') => Ok(Command::SetAlternateBuffer(false)),
            _ => Err(ParserError::Unhandled),
        }
    }

    fn read_exclamation(&self, b: u8) -> Result<Command, ParserError> {
        // @mark: ESC [ !
        match b {
            b'p' => Ok(Command::SoftReset),
            _ => Err(ParserError::Unhandled),
        }
    }

    fn read_screen_mode(&self, b: u8) -> Result<Command, ParserError> {
        // @mark: ESC [ = <n>
        match b {
            b'h' => match self.try_get_numbers(1)?.first().unwrap() {
                7 => Ok(Command::SetLineWrapping(true)),
                n => Ok(Command::SetScreenMode(self.try_read_screen_mode(*n)?)),
            },
            b'l' => match self.try_get_numbers(1)?.first().unwrap() {
                7 => Ok(Command::SetLineWrapping(false)),
                n => Ok(Command::ResetScreenMode(self.try_read_screen_mode(*n)?)),
            },
            _ => Err(ParserError::Unhandled),
        }
    }

    fn read_designate(&mut self, b: u8) -> Result<Command, ParserError> {
        // @mark: ESC (
        match b {
            b'0' => Ok(Command::SetCharacterSet(CharacterSet::LineDrawing)),
            b'B' => Ok(Command::SetCharacterSet(CharacterSet::Ascii)),
            _ => Err(ParserError::Unhandled),
        }
    }

    fn read_operating_system_command(&mut self, b: u8) -> Result<Command, ParserError> {
        // @mark: ESC ] <n> <string> <terminator>
        let Some(n) = self.numbers.first() else {
            return Err(ParserError::Unhandled);
        };

        const CHAR_BELL: u8 = 7u8;
        type Terminator = OperatingSystemCommandTerminator;
        let is_terminated = match self.osc_terminator {
            Terminator::Bell => {
                match b {
                    CHAR_BELL => true,
                    VT100_ESCAPE_CODE => {
                        self.osc_terminator = OperatingSystemCommandTerminator::Backslash;
                        false
                    },
                    _ => false,
                }
            },
            Terminator::Backslash => {
                match b {
                    b'\\' => true,
                    _ => false,
                }
            },
        };
        if !is_terminated {
            return Err(ParserError::Pending);
        }

        let total_terminator_bytes = match self.osc_terminator {
            Terminator::Bell => 1,
            Terminator::Backslash => 2, 
        };
        let i_start = self.numbers_last_index.unwrap();
        let i_end = self.buffer.len()-total_terminator_bytes;
        let data = &self.buffer[i_start..i_end];
        match n {
            0 | 2 => Ok(Command::SetWindowTitle(data)),
            8 => match data.iter().position(|b| b == &b';') {
                Some(i_delim) => {
                    let tag = &data[0..i_delim];
                    let link = &data[(i_delim+1)..];
                    Ok(Command::SetHyperlink { tag, link })
                },
                None => Ok(Command::SetHyperlink { tag: &[], link: data }),
            },
            _ => Err(ParserError::Unhandled),
        }
    }
 
    // read number list
    fn read_numbers(&mut self, b: u8) -> Result<Command, ParserError> {
        // @mark: <n>
        if b.is_ascii_digit() {
            let index = self.buffer.len()-1;
            if let Some(ref mut number_slice) = self.number_slice.as_mut() {
                number_slice.end_index = index;
            } else {
                self.number_slice = Some(NumberSlice::new(index));
            }
            return Err(ParserError::Pending);
        }
        // determine number
        if let Some(number_slice) = self.number_slice.as_ref() {
            assert!(number_slice.start_index <= number_slice.end_index);
            assert!(number_slice.end_index < self.buffer.len());
            let mut number: usize = 0;
            let mut digit_power: usize = 1;
            let number_data = &self.buffer[number_slice.start_index..=number_slice.end_index];
            for b in number_data.iter().rev() {
                let digit = (b - b'0') as usize;
                let value = digit*digit_power;
                number += value;
                digit_power *= 10;
            }
            const VT100_MIN_NUMBER: usize = 0;
            const VT100_MAX_NUMBER: usize = 32767;
            let number = number.clamp(VT100_MIN_NUMBER, VT100_MAX_NUMBER);
            self.numbers.push(number as u16);
        }
        self.number_slice = None;
        self.numbers_last_index = Some(self.buffer.len()-1);
        // @mark: <n> ;
        if b == b';' {
            Err(ParserError::Pending)
        } else {
            self.state = ParserState::Characters;
            self.parse_byte(b)
        }
    }

    // interpret number list
    fn read_optional_nonzero_u16(&self) -> NonZeroU16 {
        if self.numbers.len() > 1 {
            log::warn!("expected optional number got {} numbers ({:?})", self.numbers.len(), self);
        }
        let n = self.numbers.first().copied();
        let n = n.unwrap_or(1).max(1);
        NonZeroU16::new(n).unwrap()
    }

    fn try_read_erase_mode(&self) -> Result<EraseMode, ParserError> {
        let n = *self.numbers.first().unwrap_or(&0);
        match EraseMode::try_from_u16(n) {
            Some(erase_mode) => Ok(erase_mode),
            None => Err(ParserError::InvalidEraseMode(n))
        }
    }

    fn try_read_xy(&self) -> Result<Vector2<NonZeroU16>, ParserError> {
        let n = self.try_get_numbers(2)?;
        let x = n[1].max(1);
        let y = n[0].max(1);
        let x = NonZeroU16::new(x).unwrap();
        let y = NonZeroU16::new(y).unwrap();
        Ok(Vector2::new(x, y))
    }

    fn try_read_graphics_command(&mut self) -> Result<Command, ParserError> {
        let header = (
            self.numbers.first().to_owned(), 
            self.numbers.get(1).to_owned(),
            self.numbers.len(),
        );
        match header {
            (Some(38), Some(5), 3..) => {
                // 8bit colours
                // @mark: ESC [ 38 ; 5 ; <ID> m
                let id = self.numbers[2].min(255) as u8;
                return Ok(Command::SetForegroundColourTable(id));
            },
            (Some(48), Some(5), 3..) => {
                // 8bit colours
                // @mark: ESC [ 48 ; 5 ; <ID> m
                let id = self.numbers[2].min(255) as u8;
                return Ok(Command::SetBackgroundColourTable(id));
            },
            (Some(38), Some(2), 5..) => {
                // RGB colours
                // @mark: ESC [ 38 ; 2 ; <r> ; <g> ; <b> m
                let r = self.numbers[2].min(255) as u8;
                let g = self.numbers[3].min(255) as u8;
                let b = self.numbers[4].min(255) as u8;
                return Ok(Command::SetForegroundColourRgb(Rgb8 { r, g, b }));
            },
            (Some(48), Some(2), 5..) => {
                // RGB colours
                // @mark: ESC [ 48 ; 2 ; <r> ; <g> ; <b> m
                let r = self.numbers[2].min(255) as u8;
                let g = self.numbers[3].min(255) as u8;
                let b = self.numbers[4].min(255) as u8;
                return Ok(Command::SetBackgroundColourRgb(Rgb8 { r, g, b }));
            },
            _ => {},
        }
 
        // @mark: ESC [ <n> m
        self.graphic_styles.clear();
        if self.numbers.is_empty() {
            self.graphic_styles.push(GraphicStyle::ResetAll);
        } else {
            for n in &self.numbers {
                if let Some(g) = GraphicStyle::try_from_u16(*n) {
                    self.graphic_styles.push(g);
                } else {
                    log::warn!("Unknown graphics style {}", n);
                }
            }
        }
        if self.graphic_styles.is_empty() {
            return Err(ParserError::NoValidGraphicStyles);
        }
        Ok(Command::SetGraphicStyles(self.graphic_styles.as_slice()))
    }

    fn read_scrolling_region(&self) -> Option<ScrollRegion> {
        match (self.numbers.first(), self.numbers.get(1)) {
            (Some(top), Some(bottom)) => Some(ScrollRegion::new(*top, *bottom)),
            _ => None,
        }
    }

    fn try_read_screen_mode(&self, n: u16) -> Result<ScreenMode, ParserError> {
        let Some(mode) = ScreenMode::try_from_u16(n) else {
            return Err(ParserError::InvalidScreenMode(n));
        };
        Ok(mode)
    }

    fn try_get_numbers(&self, expected: usize) -> Result<&'_ [u16], ParserError> {
        let given = self.numbers.len();
        if given < expected {
            return Err(ParserError::MissingNumbers { given, expected });
        }
        if given > expected {
            log::warn!("expected {} numbers but got {} ({:?})", expected, given, self);
        }
        Ok(&self.numbers[0..expected])
    }
}

impl fmt::Debug for Parser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("Vt100Parser");
        res.field("state", &self.state);
        res.field("context", &self.context);
        if let Ok(ref str) = std::str::from_utf8(self.buffer.as_slice()) {
            res.field("buffer", str);
        } else {
            res.field("buffer", &self.buffer);
        };
        res.field("numbers", &self.numbers);
        res.field("number_slice", &self.number_slice);
        res.field("numbers_last_index", &self.numbers_last_index);
        res.field("graphic_styles", &self.graphic_styles);
        res.field("osc_terminator", &self.osc_terminator);
        res.finish()
    }
}
