// Sources: https://github.com/0x5c/VT100-Examples/blob/master/vt_seq.md
//          https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797
//          https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Functions-using-CSI-_-ordered-by-the-final-character_s_
use std::string::FromUtf8Error;
use cgmath::Vector2;
use crate::command::Command;
use crate::common::{
    BellVolume,
    CharacterSet,
    CursorStyle,
    EraseMode,
    GraphicStyle,
    Rgb8,
    ScreenMode,
    ScrollRegion,
    WindowAction,
};
use crate::encoder::{
    InputMode,
    KeyType,
    MouseTrackingMode,
    MouseCoordinateFormat,
};

pub const VT100_ESCAPE_CODE: u8 = 0x1B;

#[derive(Clone,Debug,PartialEq)]
pub enum ParserError {
    Unhandled,
    MissingNumbers { given: usize, expected: usize },
    InvalidEraseMode(u16),
    InvalidScreenMode(u16),
    InvalidGraphicStyle(u16),
    InvalidUtf8String(FromUtf8Error),
    InvalidKeyType(u16),
    InvalidCursorStyle(u16),
    InvalidWarningBellVolume(u16),
    InvalidMarginBellVolume(u16),
    InvalidDesignate(u8),
}

pub trait ParserHandler {
    fn on_command(&mut self, command: Command);
    fn on_error(&mut self, error: ParserError, parser: &Parser);
}

#[derive(Clone,Copy,Debug,Default,PartialEq)]
enum ParserContext {
    #[default]
    EntryPoint,                         // ESC
    ControlSequenceIntroducer,          // ESC [
    ControlSequenceIntroducerNumbers,   // ESC [ <n>
    ControlSequenceIntroducerSpace,     // ESC [ <n> <space>
    CommonPrivateMode,                  // ESC [ ?
    Exclamation,                        // ESC [ !
    ScreenMode,                         // ESC [ =
    KeyModifierOptions,                 // ESC [ >
    Designate,                          // ESC (
    OperatingSystemCommand,             // ESC ]
}

#[derive(Clone,Copy,Debug,Default,PartialEq)]
enum ParserState {
    #[default]
    Characters,
    Numbers,
    Terminated,
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
        self.osc_terminator = OperatingSystemCommandTerminator::default();
    }

    pub fn feed_byte(&mut self, b: u8, h: &mut impl ParserHandler) {
        self.buffer.push(b);
        self.parse_byte(b,h);
    }

    pub fn is_terminated(&self) -> bool {
        self.state == ParserState::Terminated
    }

    fn parse_byte(&mut self, b: u8, h: &mut impl ParserHandler) {
        match self.state {
            ParserState::Characters => {
                match self.context {
                    ParserContext::EntryPoint => self.read_entry_point(b,h),
                    ParserContext::ControlSequenceIntroducer => self.read_control_sequence_introducer(b,h),
                    ParserContext::ControlSequenceIntroducerNumbers => self.read_control_sequence_introducer_numbers(b,h),
                    ParserContext::ControlSequenceIntroducerSpace => self.read_control_sequence_introducer_space(b,h),
                    ParserContext::CommonPrivateMode => self.read_common_private_mode(b,h),
                    ParserContext::Exclamation => self.read_exclamation(b,h),
                    ParserContext::ScreenMode => self.read_screen_mode(b,h),
                    ParserContext::KeyModifierOptions => self.read_key_modifier_options(b,h),
                    ParserContext::Designate => self.read_designate(b,h),
                    ParserContext::OperatingSystemCommand => self.read_operating_system_command(b,h),
                }
            },
            ParserState::Numbers => self.read_numbers(b,h),
            ParserState::Terminated => panic!("Cannot use a terminated parser until it has been reset"),
        }
    }

    fn read_entry_point(&mut self, b: u8, h: &mut impl ParserHandler) {
        // @mark: ESC
        match b {
            b'A' => self.on_success(h, Command::MoveCursorUp(1)),
            b'B' => self.on_success(h, Command::MoveCursorDown(1)),
            b'C' => self.on_success(h, Command::MoveCursorRight(1)),
            b'D' => self.on_success(h, Command::MoveCursorLeft(1)),
            b'E' => self.on_success(h, Command::MoveCursorReverseIndex),
            b'7' => self.on_success(h, Command::SaveCursorToMemory),
            b'8' => self.on_success(h, Command::RestoreCursorFromMemory),
            b'=' => self.on_success(h, Command::SetKeypadMode(InputMode::Application)),
            b'>' => self.on_success(h, Command::SetKeypadMode(InputMode::Numeric)),
            b'H' => self.on_success(h, Command::SetTabStopAtCurrentColumn),
            b'M' => self.on_success(h, Command::MoveCursorUp(1)),
            b'[' => { 
                self.context = ParserContext::ControlSequenceIntroducer; 
                self.state = ParserState::Characters; 
            },
            b'(' => { 
                self.context = ParserContext::Designate; 
                self.state = ParserState::Characters; 
            },
            b']' => {
                self.context = ParserContext::OperatingSystemCommand;
                self.state = ParserState::Numbers; 
            },
            _ => self.on_error(h, ParserError::Unhandled),
        }
    }

    fn read_control_sequence_introducer(&mut self, b: u8, h: &mut impl ParserHandler) {
        // @mark: ESC [
        match b {
            b's' => self.on_success(h, Command::SaveCursorToMemory),
            b'u' => self.on_success(h, Command::RestoreCursorFromMemory),
            b'?' => {
                self.context = ParserContext::CommonPrivateMode;
                self.state = ParserState::Numbers; 
            },
            b'!' => {
                self.context = ParserContext::Exclamation;
                self.state = ParserState::Characters; 
            },
            b'=' => {
                self.context = ParserContext::ScreenMode;
                self.state = ParserState::Numbers;
            },
            b'>' => {
                self.context = ParserContext::KeyModifierOptions;
                self.state = ParserState::Numbers;
            },
            _ => {
                self.context = ParserContext::ControlSequenceIntroducerNumbers;
                self.state = ParserState::Numbers; 
                self.parse_byte(b,h);
            },
        }
    }

    fn read_control_sequence_introducer_numbers(&mut self, b: u8, h: &mut impl ParserHandler) {
        // @mark: ESC [ <n>
        match b {
            b'A' => self.on_success(h, Command::MoveCursorUp(self.read_optional_nonzero_u16())),
            b'B' => self.on_success(h, Command::MoveCursorDown(self.read_optional_nonzero_u16())),
            b'C' => self.on_success(h, Command::MoveCursorRight(self.read_optional_nonzero_u16())),
            b'D' => self.on_success(h, Command::MoveCursorLeft(self.read_optional_nonzero_u16())),
            b'E' => self.on_success(h, Command::MoveCursorNextLine(self.read_optional_nonzero_u16())),
            b'F' => self.on_success(h, Command::MoveCursorPreviousLine(self.read_optional_nonzero_u16())),
            b'G' => self.on_success(h, Command::MoveCursorHorizontalAbsolute(self.read_optional_nonzero_u16())),
            b'd' => self.on_success(h, Command::MoveCursorVerticalAbsolute(self.read_optional_nonzero_u16())),
            b'S' => self.on_success(h, Command::ScrollUp(self.read_optional_nonzero_u16())),
            b'T' => self.on_success(h, Command::ScrollDown(self.read_optional_nonzero_u16())),
            b'@' => self.on_success(h, Command::InsertSpaces(self.read_optional_nonzero_u16())),
            b'P' => self.on_success(h, Command::DeleteCharacters(self.read_optional_nonzero_u16())),
            b'X' => self.on_success(h, Command::ReplaceWithSpaces(self.read_optional_nonzero_u16())),
            b'L' => self.on_success(h, Command::InsertLines(self.read_optional_nonzero_u16())),
            b'M' => self.on_success(h, Command::DeleteLines(self.read_optional_nonzero_u16())),
            b'J' => self.on_result(h, self.try_read_erase_mode().map(Command::EraseInDisplay)),
            b'K' => self.on_result(h, self.try_read_erase_mode().map(Command::EraseInLine)),
            b'H' => {
                let pos = self.try_read_xy().unwrap_or(Vector2::new(1,1));
                self.on_success(h, Command::MoveCursorPositionViewport(pos));
            },
            b'f' => self.on_result(h, self.try_read_xy().map(Command::MoveCursorPositionViewport)),
            b'm' => self.read_graphics_command(h),
            b'r' => self.on_success(h, Command::SetScrollRegion(self.read_scrolling_region())),
            b'I' => self.on_success(h, Command::AdvanceCursorToTabStop(self.read_optional_nonzero_u16())),
            b'Z' => self.on_success(h, Command::ReverseCursorToTabStop(self.read_optional_nonzero_u16())),
            b'g' => match self.try_get_numbers(1).map(|v| v[0]) {
                Err(err) => self.on_error(h, err),
                Ok(0) => self.on_success(h, Command::ClearCurrentTabStop),
                Ok(3) => self.on_success(h, Command::ClearAllTabStops),
                _ => self.on_error(h, ParserError::Unhandled),
            },
            b'n' => match self.try_get_numbers(1).map(|v| v[0]) {
                Err(err) => self.on_error(h, err),
                Ok(6) => self.on_success(h, Command::QueryCursorPosition),
                _ => self.on_error(h, ParserError::Unhandled),
            },
            b'c' => match self.numbers.first().copied().unwrap_or(0) {
                0 => self.on_success(h, Command::QueryTerminalIdentity),
                id => self.on_success(h, Command::UnhandledDeviceQuery(id)),
            },
            b't' => self.read_window_action(h),
            // reset/set modes (this is different but similar to ESC [ ? <n> h/l
            b'h' => match self.try_get_numbers(1).map(|v| v[0]) {
                Err(err) => self.on_error(h, err),
                Ok(2) => self.on_success(h, Command::SetKeyboardActionMode(true)),
                Ok(4) => self.on_success(h, Command::SetInsertMode),
                Ok(20) => self.on_success(h, Command::SetAutomaticNewline),
                _ => self.on_error(h, ParserError::Unhandled),
            },
            b'l' => match self.try_get_numbers(1).map(|v| v[0]) {
                Err(err) => self.on_error(h, err),
                Ok(2) => self.on_success(h, Command::SetKeyboardActionMode(false)),
                Ok(4) => self.on_success(h, Command::SetReplaceMode),
                Ok(20) => self.on_success(h, Command::SetNormalLinefeed),
                _ => self.on_error(h, ParserError::Unhandled),
            },
            b' ' => {
                self.context = ParserContext::ControlSequenceIntroducerSpace;
                self.state = ParserState::Characters;
            },
            _ => self.on_error(h, ParserError::Unhandled),
        }
    }

    fn read_control_sequence_introducer_space(&mut self, b: u8, h: &mut impl ParserHandler) {
        // @mark: ESC [ <n> <space>
        match b {
            b'@' => {
                let n = self.numbers.first().copied().unwrap_or(1).max(1);
                self.on_success(h, Command::ShiftLeftByColumns(n));
            },
            b'A' => {
                let n = self.numbers.first().copied().unwrap_or(1).max(1);
                self.on_success(h, Command::ShiftRightByColumns(n));
            },
            b'q' => match self.numbers.first().copied().unwrap_or(1) {
                0 | 1 => {
                    self.on_success(h, Command::SetCursorBlinking(true));
                    self.on_success(h, Command::SetCursorStyle(CursorStyle::Block));
                },
                2 => {
                    self.on_success(h, Command::SetCursorBlinking(false));
                    self.on_success(h, Command::SetCursorStyle(CursorStyle::Block));
                },
                3 => {
                    self.on_success(h, Command::SetCursorBlinking(true));
                    self.on_success(h, Command::SetCursorStyle(CursorStyle::Underline));
                },
                4 => {
                    self.on_success(h, Command::SetCursorBlinking(false));
                    self.on_success(h, Command::SetCursorStyle(CursorStyle::Underline));
                },
                5 => {
                    self.on_success(h, Command::SetCursorBlinking(true));
                    self.on_success(h, Command::SetCursorStyle(CursorStyle::Bar));
                },
                6 => {
                    self.on_success(h, Command::SetCursorBlinking(false));
                    self.on_success(h, Command::SetCursorStyle(CursorStyle::Bar));
                },
                n => self.on_error(h, ParserError::InvalidCursorStyle(n)),
            },
            b't' => match self.try_get_numbers(1).map(|v| v[0]) {
                Err(err) => self.on_error(h, err),
                Ok(v) => match v {
                    0..=1 => self.on_success(h, Command::SetWarningBellVolume(BellVolume::Off)),
                    2..=4 => self.on_success(h, Command::SetWarningBellVolume(BellVolume::Low)),
                    5..=8 => self.on_success(h, Command::SetWarningBellVolume(BellVolume::High)),
                    _ => self.on_error(h, ParserError::InvalidWarningBellVolume(v)),
                },
            },
            b'u' => match self.try_get_numbers(1).map(|v| v[0]) {
                Err(err) => self.on_error(h, err),
                Ok(v) => match v {
                    0 => self.on_success(h, Command::SetMarginBellVolume(BellVolume::High)),
                    1 => self.on_success(h, Command::SetMarginBellVolume(BellVolume::Off)),
                    2..=4 => self.on_success(h, Command::SetMarginBellVolume(BellVolume::Low)),
                    5..=8 => self.on_success(h, Command::SetMarginBellVolume(BellVolume::High)),
                    _ => self.on_error(h, ParserError::InvalidMarginBellVolume(v)),
                },
            },
            _ => self.on_error(h, ParserError::Unhandled),
        }
    }

    fn read_common_private_mode(&mut self, b: u8, h: &mut impl ParserHandler) {
        // @mark: ESC [ ? <n>
        if self.numbers.is_empty() {
            self.on_error(h, ParserError::MissingNumbers { given: 0, expected: 1 });
            return;
        }

        let total_numbers = self.numbers.len();
        for i in 0..total_numbers {
            let n = self.numbers[i];
            match (n, b) {
                (   1, b'h') => self.on_success(h, Command::SetCursorKeyInputMode(InputMode::Application)),
                (   1, b'l') => self.on_success(h, Command::SetCursorKeyInputMode(InputMode::Numeric)),
                (   3, b'h') => self.on_success(h, Command::SetConsoleWidth(132)),
                (   3, b'l') => self.on_success(h, Command::SetConsoleWidth(80)),
                (   5, b'h') => self.on_success(h, Command::SetLightBackground),
                (   5, b'l') => self.on_success(h, Command::SetDarkBackground),
                (   9, b'h') => {
                    self.on_success(h, Command::SetMouseTrackingMode(MouseTrackingMode::X10));
                    self.on_success(h, Command::SetMouseCoordinateFormat(MouseCoordinateFormat::X10));
                },
                (   9, b'l') => self.on_success(h, Command::SetMouseTrackingMode(MouseTrackingMode::Disabled)),
                (  12, b'h') => self.on_success(h, Command::SetCursorBlinking(true)),
                (  12, b'l') => self.on_success(h, Command::SetCursorBlinking(false)),
                (  25, b'h') => self.on_success(h, Command::SetCursorVisible(true)),
                (  25, b'l') => self.on_success(h, Command::SetCursorVisible(false)),
                (  47, b'h') => self.on_success(h, Command::SetAlternateBuffer(true)),
                (  47, b'l') => self.on_success(h, Command::SetAlternateBuffer(false)),
                (1000, b'h') => self.on_success(h, Command::SetMouseTrackingMode(MouseTrackingMode::Normal)),
                (1000, b'l') => self.on_success(h, Command::SetMouseTrackingMode(MouseTrackingMode::Disabled)),
                (1001, b'h') => self.on_success(h, Command::SetMouseTrackingMode(MouseTrackingMode::Highlight)),
                (1001, b'l') => self.on_success(h, Command::SetMouseTrackingMode(MouseTrackingMode::Disabled)),
                (1002, b'h') => self.on_success(h, Command::SetMouseTrackingMode(MouseTrackingMode::Motion)),
                (1002, b'l') => self.on_success(h, Command::SetMouseTrackingMode(MouseTrackingMode::Disabled)),
                (1003, b'h') => self.on_success(h, Command::SetMouseTrackingMode(MouseTrackingMode::Any)),
                (1003, b'l') => self.on_success(h, Command::SetMouseTrackingMode(MouseTrackingMode::Disabled)),
                (1004, b'h') => self.on_success(h, Command::SetReportFocus(true)),
                (1004, b'l') => self.on_success(h, Command::SetReportFocus(false)),
                (1005, b'h') => self.on_success(h, Command::SetMouseCoordinateFormat(MouseCoordinateFormat::Utf8)),
                (1005, b'l') => self.on_success(h, Command::SetMouseCoordinateFormat(MouseCoordinateFormat::X10)),
                (1006, b'h') => self.on_success(h, Command::SetMouseCoordinateFormat(MouseCoordinateFormat::Sgr)),
                (1006, b'l') => self.on_success(h, Command::SetMouseCoordinateFormat(MouseCoordinateFormat::X10)),
                (1015, b'h') => self.on_success(h, Command::SetMouseCoordinateFormat(MouseCoordinateFormat::Urxvt)),
                (1015, b'l') => self.on_success(h, Command::SetMouseCoordinateFormat(MouseCoordinateFormat::X10)),
                (1016, b'h') => self.on_success(h, Command::SetMouseCoordinateFormat(MouseCoordinateFormat::SgrPixel)),
                (1016, b'l') => self.on_success(h, Command::SetMouseCoordinateFormat(MouseCoordinateFormat::X10)),
                (1047, b'h') => self.on_success(h, Command::SetAlternateBuffer(true)),
                (1047, b'l') => self.on_success(h, Command::SetAlternateBuffer(false)),
                (1048, b'h') => self.on_success(h, Command::SaveCursorToMemory),
                (1048, b'l') => self.on_success(h, Command::RestoreCursorFromMemory),
                (1049, b'h') => {
                    self.on_success(h, Command::SaveCursorToMemory);
                    self.on_success(h, Command::SetAlternateBuffer(true));
                },
                (1049, b'l') => {
                    self.on_success(h, Command::SetAlternateBuffer(false));
                    self.on_success(h, Command::RestoreCursorFromMemory);
                },
                (2004, b'h') => self.on_success(h, Command::SetBracketedPasteMode(true)),
                (2004, b'l') => self.on_success(h, Command::SetBracketedPasteMode(false)),
                (code, b'h') => self.on_success(h, Command::UnhandledPrivateMode(code, true)),
                (code, b'l') => self.on_success(h, Command::UnhandledPrivateMode(code, false)),
                (   n, b'm') => match KeyType::try_from_u16(n) {
                    Some(key_type) => self.on_success(h, Command::QueryKeyModifierOption(key_type)),
                    None => self.on_error(h, ParserError::InvalidKeyType(n)),
                },
                _ => self.on_error(h, ParserError::Unhandled),
            }
        }
    }

    fn read_exclamation(&mut self, b: u8, h: &mut impl ParserHandler) {
        // @mark: ESC [ !
        match b {
            b'p' => self.on_success(h, Command::SoftReset),
            _ => self.on_error(h, ParserError::Unhandled),
        }
    }

    fn read_screen_mode(&mut self, b: u8, h: &mut impl ParserHandler) {
        // @mark: ESC [ = <n>
        match b {
            b'h' => match self.try_get_numbers(1).map(|v| v[0]) {
                Err(err) => self.on_error(h, err),
                Ok(7) => self.on_success(h, Command::SetLineWrapping(true)),
                Ok(n) => self.on_result(h, self.try_read_screen_mode(n).map(Command::SetScreenMode)),
            },
            b'l' => match self.try_get_numbers(1).map(|v| v[0]) {
                Err(err) => self.on_error(h, err),
                Ok(7) => self.on_success(h, Command::SetLineWrapping(false)),
                Ok(n) => self.on_result(h, self.try_read_screen_mode(n).map(Command::ResetScreenMode)),
            },
            _ => self.on_error(h, ParserError::Unhandled),
        }
    }

    fn read_key_modifier_options(&mut self, b: u8, h: &mut impl ParserHandler) {
        // @mark: ESC [ > <n>
        match b {
            b'm' => match self.numbers.first().copied() {
                Some(n) => match KeyType::try_from_u16(n) {
                    Some(key_type) => {
                        let value = self.numbers.get(1).copied();
                        self.on_success(h, Command::SetKeyModifierOption(key_type, value));
                    },
                    None => self.on_error(h, ParserError::InvalidKeyType(n)),
                },
                None => self.on_error(h, ParserError::MissingNumbers { given: 0, expected: 1 }),
            },
            b'c' => match self.numbers.first().copied() {
                Some(0) | None => self.on_success(h, Command::QueryTerminalIdentity),
                _ => self.on_error(h, ParserError::Unhandled),
            },
            _ => self.on_error(h, ParserError::Unhandled),
        }
    }

    fn read_designate(&mut self, b: u8, h: &mut impl ParserHandler) {
        // @mark: ESC (
        match b {
            b'0' => self.on_success(h, Command::SetCharacterSet(CharacterSet::LineDrawing)),
            b'B' => self.on_success(h, Command::SetCharacterSet(CharacterSet::Ascii)),
            _ => self.on_error(h, ParserError::InvalidDesignate(b)),
        }
    }

    fn read_operating_system_command(&mut self, b: u8, h: &mut impl ParserHandler) {
        // @mark: ESC ] <n> <string> <terminator>
        let n = match self.try_get_numbers(1).map(|v| v[0]) {
            Ok(v) => v,
            Err(err) => return self.on_error(h, err),
        };

        const CHAR_BELL: u8 = 7u8;
        type Terminator = OperatingSystemCommandTerminator;
        // terminator can be BELL or ESC\
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
            Terminator::Backslash => b == b'\\',
        };
        if !is_terminated {
            return;
        }

        let total_terminator_bytes = match self.osc_terminator {
            Terminator::Bell => 1,
            Terminator::Backslash => 2,
        };
        let i_start = self.numbers_last_index.unwrap();
        let i_end = self.buffer.len()-total_terminator_bytes;
        let data = &self.buffer[i_start..i_end];
        match n {
            0 | 2 => match String::from_utf8(data.to_vec()) {
                Ok(title) => self.on_success(h, Command::WindowAction(WindowAction::SetWindowTitle(title))),
                Err(error) => self.on_error(h, ParserError::InvalidUtf8String(error)),
            },
            8 => match String::from_utf8(data.to_vec()) {
                Ok(title) => self.on_success(h, Command::SetHyperlink(title)),
                Err(error) => self.on_error(h, ParserError::InvalidUtf8String(error)),
            },
            _ => self.on_success(h, Command::UnhandledOperatingSystemCommand(n, data.to_vec())),
        }
    }
 
    // read number list
    fn read_numbers(&mut self, b: u8, h: &mut impl ParserHandler) {
        // @mark: <n>
        if b.is_ascii_digit() {
            let index = self.buffer.len()-1;
            if let Some(ref mut number_slice) = self.number_slice.as_mut() {
                number_slice.end_index = index;
            } else {
                self.number_slice = Some(NumberSlice::new(index));
            }
            return;
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
        // @mark: <n> :
        if [b';', b':'].contains(&b) {
            return;
        }
        self.state = ParserState::Characters;
        self.parse_byte(b,h); // try to re-parse as character
    }

    // interpret number list
    fn read_optional_nonzero_u16(&self) -> u16 {
        if self.numbers.len() > 1 {
            log::warn!("expected optional number got {} numbers ({:?})", self.numbers.len(), self);
        }
        let n = self.numbers.first().copied();
        n.unwrap_or(1).max(1)
    }

    fn try_read_erase_mode(&self) -> Result<EraseMode, ParserError> {
        let n = *self.numbers.first().unwrap_or(&0);
        match EraseMode::try_from_u16(n) {
            Some(erase_mode) => Ok(erase_mode),
            None => Err(ParserError::InvalidEraseMode(n))
        }
    }

    fn try_read_xy(&self) -> Result<Vector2<u16>, ParserError> {
        let n = self.try_get_numbers(2)?;
        let x = n[1].max(1);
        let y = n[0].max(1);
        Ok(Vector2::new(x,y))
    }

    fn read_graphics_command(&mut self, h: &mut impl ParserHandler) {
        // @mark: ESC [ <n> m
        match self.numbers.as_slice() {
            [38,5,id,..] => return self.on_success(h, Command::SetForegroundColourTable((*id).min(255) as u8)),
            [48,5,id,..] => return self.on_success(h, Command::SetBackgroundColourTable((*id).min(255) as u8)),
            [38,2,r,g,b,..] => {
                let rgb = Rgb8 {
                    r: (*r).min(255) as u8,
                    g: (*g).min(255) as u8,
                    b: (*b).min(255) as u8,
                };
                return self.on_success(h, Command::SetForegroundColourRgb(rgb));
            },
            [48,2,r,g,b,..] => {
                let rgb = Rgb8 {
                    r: (*r).min(255) as u8,
                    g: (*g).min(255) as u8,
                    b: (*b).min(255) as u8,
                };
                return self.on_success(h, Command::SetBackgroundColourRgb(rgb));
            },
            _ => {},
        }
 
        if self.numbers.is_empty() {
            return self.on_success(h, Command::SetGraphicStyle(GraphicStyle::ResetAll));
        }
 
        let total_numbers = self.numbers.len();
        for i in 0..total_numbers {
            let n = self.numbers[i];
            match GraphicStyle::try_from_u16(n) {
                Some(style) => self.on_success(h, Command::SetGraphicStyle(style)),
                None => self.on_error(h, ParserError::InvalidGraphicStyle(n)),
            }
        }
    }

    fn read_scrolling_region(&self) -> Option<ScrollRegion> {
        match self.numbers.as_slice() {
            [top, bottom, ..] => Some(ScrollRegion::new(*top, *bottom)),
            _ => None,
        }
    }

    fn try_read_screen_mode(&self, n: u16) -> Result<ScreenMode, ParserError> {
        match ScreenMode::try_from_u16(n) {
            Some(mode) => Ok(mode),
            None => Err(ParserError::InvalidScreenMode(n)),
        }
    }

    fn read_window_action(&mut self, h: &mut impl ParserHandler) {
        // @mark: ESC [ <n> t
        let Some(code) = self.numbers.first().copied() else {
            self.on_error(h, ParserError::MissingNumbers { given: 0, expected: 1 });
            return;
        };
        match code {
            1 => self.on_success(h, Command::WindowAction(WindowAction::SetMinimised(false))),
            2 => self.on_success(h, Command::WindowAction(WindowAction::SetMinimised(true))),
            3 => match self.try_get_numbers(3) {
                Err(err) => self.on_error(h, err),
                Ok(v) => self.on_success(h, Command::WindowAction(WindowAction::Move(Vector2::new(v[1],v[2])))),
            },
            4 => match self.try_get_numbers(3) {
                Err(err) => self.on_error(h, err),
                Ok(v) => self.on_success(h, Command::WindowAction(WindowAction::Resize(Vector2::new(v[1],v[2])))),
            },
            5 => self.on_success(h, Command::WindowAction(WindowAction::SendToFront)),
            6 => self.on_success(h, Command::WindowAction(WindowAction::SendToBack)),
            7 => self.on_success(h, Command::WindowAction(WindowAction::Refresh)),
            8 => match self.try_get_numbers(3) {
                Err(err) => self.on_error(h, err),
                Ok(v) => self.on_success(h, Command::WindowAction(WindowAction::ResizeTextArea(Vector2::new(v[1],v[2])))),
            },
            9 => match self.numbers.get(1).copied() {
                None => self.on_error(h, ParserError::MissingNumbers { given: self.numbers.len(), expected: 2 }),
                Some(0) => self.on_success(h, Command::WindowAction(WindowAction::RestoreMaximised)),
                Some(1) => self.on_success(h, Command::WindowAction(WindowAction::Maximise(Vector2::new(true, true)))),
                Some(2) => self.on_success(h, Command::WindowAction(WindowAction::Maximise(Vector2::new(false, true)))),
                Some(3) => self.on_success(h, Command::WindowAction(WindowAction::Maximise(Vector2::new(true, false)))),
                _ => self.on_error(h, ParserError::Unhandled),
            },
            10 => match self.numbers.get(1).copied() {
                None => self.on_error(h, ParserError::MissingNumbers { given: self.numbers.len(), expected: 2 }),
                Some(0) => self.on_success(h, Command::WindowAction(WindowAction::SetFullscreen(false))),
                Some(1) => self.on_success(h, Command::WindowAction(WindowAction::SetFullscreen(true))),
                Some(2) => self.on_success(h, Command::WindowAction(WindowAction::ToggleFullscreen)),
                _ => self.on_error(h, ParserError::Unhandled),
            },
            11 => self.on_success(h, Command::WindowAction(WindowAction::GetWindowState)),
            13 => match self.numbers.get(1).copied() {
                None => self.on_success(h, Command::WindowAction(WindowAction::GetWindowPosition)),
                Some(2) => self.on_success(h, Command::WindowAction(WindowAction::GetTextAreaPosition)),
                _ => self.on_error(h, ParserError::Unhandled),
            },
            14 => match self.numbers.get(1).copied() {
                None => self.on_success(h, Command::WindowAction(WindowAction::GetTextAreaSize)),
                Some(2) => self.on_success(h, Command::WindowAction(WindowAction::GetWindowSize)),
                _ => self.on_error(h, ParserError::Unhandled),
            },
            15 => self.on_success(h, Command::WindowAction(WindowAction::GetScreenSize)),
            16 => self.on_success(h, Command::WindowAction(WindowAction::GetCellSize)),
            18 => self.on_success(h, Command::WindowAction(WindowAction::GetTextAreaGridSize)),
            19 => self.on_success(h, Command::WindowAction(WindowAction::GetScreenGridSize)),
            20 => self.on_success(h, Command::WindowAction(WindowAction::GetWindowIconLabel)),
            21 => self.on_success(h, Command::WindowAction(WindowAction::GetWindowTitle)),
            22 => match self.numbers.get(1).copied() {
                None => self.on_error(h, ParserError::MissingNumbers { given: self.numbers.len(), expected: 2 }),
                Some(0) => {
                    let stack_index = self.numbers.get(2).copied();
                    self.on_success(h, Command::WindowAction(WindowAction::SaveIconTitle(stack_index)));
                    self.on_success(h, Command::WindowAction(WindowAction::SaveWindowTitle(stack_index)));
                },
                Some(1) => {
                    let stack_index = self.numbers.get(2).copied();
                    self.on_success(h, Command::WindowAction(WindowAction::SaveIconTitle(stack_index)));
                },
                Some(2) => {
                    let stack_index = self.numbers.get(2).copied();
                    self.on_success(h, Command::WindowAction(WindowAction::SaveWindowTitle(stack_index)));
                },
                _ => self.on_error(h, ParserError::Unhandled),
            },
            23 => match self.numbers.get(1).copied() {
                None => self.on_error(h, ParserError::MissingNumbers { given: self.numbers.len(), expected: 2 }),
                Some(0) => {
                    let stack_index = self.numbers.get(2).copied();
                    self.on_success(h, Command::WindowAction(WindowAction::RestoreIconTitle(stack_index)));
                    self.on_success(h, Command::WindowAction(WindowAction::RestoreWindowTitle(stack_index)));
                },
                Some(1) => {
                    let stack_index = self.numbers.get(2).copied();
                    self.on_success(h, Command::WindowAction(WindowAction::RestoreIconTitle(stack_index)));
                },
                Some(2) => {
                    let stack_index = self.numbers.get(2).copied();
                    self.on_success(h, Command::WindowAction(WindowAction::RestoreWindowTitle(stack_index)));
                },
                _ => self.on_error(h, ParserError::Unhandled),
            },
            24.. => self.on_success(h, Command::WindowAction(WindowAction::ResizeWindowHeight(code))),
            _ => self.on_error(h, ParserError::Unhandled),
        }
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

    fn on_success(&mut self, h: &mut impl ParserHandler, command: Command) {
        h.on_command(command);
        self.state = ParserState::Terminated;
    }

    fn on_error(&mut self, h: &mut impl ParserHandler, error: ParserError) {
        h.on_error(error, self);
        self.state = ParserState::Terminated;
    }

    fn on_result(&mut self, h: &mut impl ParserHandler, result: Result<Command, ParserError>) {
        match result {
            Ok(command) => h.on_command(command),
            Err(error) => h.on_error(error, self),
        }
        self.state = ParserState::Terminated;
    }
}

impl std::fmt::Debug for Parser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
        res.field("osc_terminator", &self.osc_terminator);
        res.finish()
    }
}
