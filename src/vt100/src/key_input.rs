use crate::misc::InputMode;

// Source: https://vt100.net/docs/vt100-ug/chapter3.html
// https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct KeyInput {
    pub input_mode: InputMode,
    pub is_modifier_shift: bool,
    pub is_modifier_ctrl: bool,
    pub is_modifier_alt: bool,
}

pub enum ArrowKey {
    Up,
    Down,
    Left,
    Right,
}

pub enum FunctionKey {
    Escape,
    Tab,
    Backspace,
    Enter,
    LineFeed,
    Delete,
}

impl KeyInput {
    pub fn on_function_key(&self, key: FunctionKey) -> &[u8] {
        // Figure C-2: Function key control codes
        match key {
            FunctionKey::Escape    => b"\x1b",
            FunctionKey::Tab       => b"\x09",
            FunctionKey::Backspace => b"\x08",
            FunctionKey::Enter     => b"\x0e",
            FunctionKey::LineFeed  => b"\x0a",
            FunctionKey::Delete    => b"\x7f",
        }
    }

    pub fn on_character_ctrl_key(&self, b: u8) -> Option<&[u8]> {
        // Figure C-2: Function key control codes
        let res: &[u8] = match b {
            b' '  => b"\x00",
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
            b'\\' => b"\x1c",
            b']'  => b"\x1d",
            b'/'  => b"\x1f",
            b'`'  => b"\x60",
            _ => return None,
        };
        Some(res)
    }

    pub fn on_arrow_key(&self, key: ArrowKey) -> &[u8] {
        match self.input_mode {
            InputMode::Application => match key {
                ArrowKey::Up => b"\x1bOA",
                ArrowKey::Down => b"\x1bOB",
                ArrowKey::Right => b"\x1bOC",
                ArrowKey::Left => b"\x1bOD",
            },
            InputMode::Numeric => {
                if self.is_modifier_ctrl {
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
        }
    }
}
