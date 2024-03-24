use bitflags::bitflags;
use cgmath::Vector2;
use std::io::Write;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum InputMode {
    Application,
    Numeric,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum KeyType {
    Keyboard,
    CursorKeys,
    FunctionKeys,
    KeypadKeys,
    OtherKeys,
    StringKeys,
}

impl KeyType {
    pub(crate) fn try_from_u16(x: u16) -> Option<Self> {
        match x {
            0 => Some(Self::Keyboard),
            1 => Some(Self::CursorKeys),
            2 => Some(Self::FunctionKeys),
            3 => Some(Self::KeypadKeys),
            4 => Some(Self::OtherKeys),
            5 => Some(Self::StringKeys),
            _ => None,
        }
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum MouseCoordinateFormat {
    // https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
    // https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Extended-coordinates
    X10,
    Normal,
    Utf8,
    Sgr,
    Urxvt,
    SgrPixel,
}

bitflags! {
    #[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
    pub struct ModifierKey: u8 {
        const None  = 0b0000_0000;
        const Ctrl  = 0b0000_0001;
        const Shift = 0b0000_0010;
        const Alt   = 0b0000_0100;
        const _ = 0u8;
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum MouseButtonEvent {
    LeftClick,
    RightClick,
    MiddleClick,
}

impl MouseButtonEvent {
    fn to_event_code(&self) -> u8 {
        match self {
            Self::LeftClick => 0,
            Self::RightClick => 1,
            Self::MiddleClick => 2,
        }
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ArrowKey {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum FunctionKey {
    Escape,
    Tab,
    Backspace,
    Enter,
    LineFeed,
    Delete,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum KeyCode {
    Char(char),
    ArrowKey(ArrowKey),
    FunctionKey(FunctionKey),
    ModifierKey(ModifierKey),
}

pub struct Encoder {
    pub is_bracketed_paste_mode: bool,
    pub modifier_key: ModifierKey,
    pub keypad_input_mode: InputMode,
    pub cursor_key_input_mode: InputMode,
    pub is_mouse_button_event: bool,
    pub is_report_focus: bool,
    utf8_encode_buffer: [u8;4],
    encode_buffer: Vec<u8>,
}

impl Default for Encoder {
    fn default() -> Self {
        Self {
            is_bracketed_paste_mode: false,
            modifier_key: ModifierKey::None,
            keypad_input_mode: InputMode::Numeric,
            cursor_key_input_mode: InputMode::Numeric,
            is_mouse_button_event: false,
            is_report_focus: false,
            utf8_encode_buffer: [0u8; 4],
            encode_buffer: Vec::new(),
        }
    }

}

impl Encoder {
    pub fn on_mouse_button(&mut self, pos: Vector2<usize>, event: MouseButtonEvent, output: &mut impl FnMut(&[u8])) {
        if !self.is_mouse_button_event {
            return;
        }
        // https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Button-event-tracking
        // TODO:
        // self.encode_buffer.clear();
        // write!(&mut self.encode_buffer, b"\x1b[M");
        // let code = event.to_event_code() + 32u8;
        // let _ = self.encode_buffer.write_all(&[code]);
        // write!(&mut self.encode_buffer, b"{}{}");
        // output(self.encode_buffer.as_slice());
    }

    pub fn on_window_focus(&self, is_focus: bool, output: &mut impl FnMut(&[u8])) {
        if !self.is_report_focus {
            return;
        }
        // https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-FocusIn_FocusOut
        if is_focus {
            output(b"\x1b[I");
        } else {
            output(b"\x1b[O");
        }
    }

    pub fn on_key_press(&mut self, key_code: KeyCode, output: &mut impl FnMut(&[u8])) {
        match key_code {
            KeyCode::Char(c) => self.on_character(c, output),
            KeyCode::ArrowKey(arrow_key) => self.on_arrow_key(arrow_key, output),
            KeyCode::FunctionKey(function_key) => self.on_function_key(function_key, output),
            KeyCode::ModifierKey(modifier_key) => self.modifier_key.insert(modifier_key),
        }
    }

    pub fn on_key_release(&mut self, key_code: KeyCode, output: &mut impl FnMut(&[u8])) {
        if let KeyCode::ModifierKey(key) = key_code {
            self.modifier_key.remove(key);
        }
    }

    fn on_character(&mut self, c: char, output: &mut impl FnMut(&[u8])) {
        let data = c.encode_utf8(&mut self.utf8_encode_buffer);
        let data = data.as_bytes();
        if self.modifier_key.contains(ModifierKey::Ctrl) && data.len() == 1 {
            if let Some(data) = Self::get_character_ctrl_key(data[0]) {
                output(data);
                return;
            }
        }
        output(data);
    }

    fn on_function_key(&mut self, key: FunctionKey, output: &mut impl FnMut(&[u8])) {
        // Figure C-2: Function key control codes
        let data = match key {
            FunctionKey::Escape    => b"\x1b",
            FunctionKey::Tab       => b"\x09",
            FunctionKey::Backspace => b"\x08",
            FunctionKey::Enter     => b"\x0d",
            FunctionKey::LineFeed  => b"\x0a",
            FunctionKey::Delete    => b"\x7f",
        };
        output(data);
    }

    fn get_character_ctrl_key(b: u8) -> Option<&'static [u8]> {
        // https://vt100.net/docs/vt220-rm/chapter3.html#T3-5
        let data: &'static [u8] = match b {
            b' '  => b"\x00",
            b'2'  => b"\x00",
            b'a'  => b"\x01",
            b'b'  => b"\x02",
            b'c'  => b"\x03",
            b'd'  => b"\x04",
            b'e'  => b"\x05",
            b'f'  => b"\x06",
            b'g'  => b"\x07",
            b'h'  => b"\x08",
            b'i'  => b"\x09",
            b'j'  => b"\x0a",
            b'k'  => b"\x0b",
            b'l'  => b"\x0c",
            b'm'  => b"\x0d",
            b'n'  => b"\x0e",
            b'o'  => b"\x0f",
            b'p'  => b"\x10",
            b'q'  => b"\x11",
            b'r'  => b"\x12",
            b's'  => b"\x13",
            b't'  => b"\x14",
            b'u'  => b"\x15",
            b'v'  => b"\x16",
            b'w'  => b"\x17",
            b'x'  => b"\x18",
            b'y'  => b"\x19",
            b'z'  => b"\x1a",
            b'['  => b"\x1b",
            b'3'  => b"\x1b",
            b'\\' => b"\x1c",
            b'4'  => b"\x1c",
            b']'  => b"\x1d",
            b'5'  => b"\x1d",
            b'`'  => b"\x1e",
            b'6'  => b"\x1e",
            b'/'  => b"\x1f",
            b'7'  => b"\x1f",
            b'8'  => b"\x7f",
            _ => return None,
        };
        Some(data)
    }

    fn on_arrow_key(&mut self, key: ArrowKey, output: &mut impl FnMut(&[u8])) {
        let data: &'static [u8] = match self.cursor_key_input_mode {
            InputMode::Application => {
                if self.modifier_key.contains(ModifierKey::Ctrl) {
                    match key {
                        ArrowKey::Up => b"\x1b[1;5;A",
                        ArrowKey::Down => b"\x1b[1;5;B",
                        ArrowKey::Right => b"\x1b[1;5;C",
                        ArrowKey::Left => b"\x1b[1;5;D",
                    }
                } else {
                    match key {
                        ArrowKey::Up => b"\x1bOA",
                        ArrowKey::Down => b"\x1bOB",
                        ArrowKey::Right => b"\x1bOC",
                        ArrowKey::Left => b"\x1bOD",
                    }
                }
            },
            InputMode::Numeric => {
                if self.modifier_key.contains(ModifierKey::Ctrl) {
                    match key {
                        ArrowKey::Up => b"\x1b[1;5;A",
                        ArrowKey::Down => b"\x1b[1;5;B",
                        ArrowKey::Right => b"\x1b[1;5;C",
                        ArrowKey::Left => b"\x1b[1;5;D",
                    }
                } else {
                    match key {
                        ArrowKey::Up => b"\x1b[A",
                        ArrowKey::Down => b"\x1b[B",
                        ArrowKey::Right => b"\x1b[C",
                        ArrowKey::Left => b"\x1b[D",
                    }
                }
            },
        };
        output(data);
    }

    pub fn paste_text(&mut self, buf: &[u8], output: &mut impl FnMut(&[u8])) {
        if self.is_bracketed_paste_mode {
            output(b"\x1b[200~"); 
            output(buf);
            output(b"\x1b[201~"); 
        } else {
            output(buf);
        }
    }

    pub fn set_window_size_characters(&mut self, size: Vector2<usize>, output: &mut impl FnMut(&[u8])) {
        // https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Miscellaneous
        self.encode_buffer.clear();
        if write!(&mut self.encode_buffer, "\x1b[18;{};{}t", size.x, size.y).is_ok() {
            output(self.encode_buffer.as_slice());
        }
    }
}
