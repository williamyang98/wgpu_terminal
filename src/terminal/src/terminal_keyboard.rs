use vt100::misc::InputMode;
use vt100::key_input::{
    ModifierKey, ArrowKey, FunctionKey,
    on_function_key,
    on_character_ctrl_key,
    on_arrow_key,
    on_bracketed_paste,
};
use std::io::Write;
use std::ops::DerefMut;

pub struct TerminalKeyboard {
    is_bracketed_paste_mode: bool,
    modifier_key: ModifierKey,
    keypad_input_mode: InputMode,
    cursor_key_input_mode: InputMode,
    write_pipe: Box<dyn Write>,
    utf8_decode_buffer: [u8;4],
}

pub enum KeyCode {
    Char(char),
    ArrowKey(ArrowKey),
    FunctionKey(FunctionKey),
    ModifierKey(ModifierKey),
}

impl TerminalKeyboard {
    pub fn new(write_pipe: Box<dyn Write>) -> Self {
        Self {
            is_bracketed_paste_mode: true,
            modifier_key: ModifierKey::None,
            keypad_input_mode: InputMode::Numeric,
            cursor_key_input_mode: InputMode::Numeric,
            write_pipe,
            utf8_decode_buffer: [0u8;4],
        }
    }

    pub fn set_keypad_input_mode(&mut self, input_mode: InputMode) {
        self.keypad_input_mode = input_mode;
    }

    pub fn set_cursor_key_input_mode(&mut self, input_mode: InputMode) {
        self.cursor_key_input_mode = input_mode;
    }

    pub fn paste_text(&mut self, text: &[u8]) {
        if self.is_bracketed_paste_mode {
            on_bracketed_paste(text, self.write_pipe.deref_mut()); 
        } else {
            let _ = self.write_pipe.write(text); 
        }
    }

    pub fn on_key_press(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Char(c) => {
                if self.modifier_key.contains(ModifierKey::Ctrl) {
                    let data = c.encode_utf8(&mut self.utf8_decode_buffer);
                    let data = data.as_bytes();
                    if data.len() == 1 {
                        if let Some(data) = on_character_ctrl_key(data[0]) {
                            let _ = self.write_pipe.write(data);
                        }
                    }
                } else {
                    let data = c.encode_utf8(&mut self.utf8_decode_buffer);
                    let _ = self.write_pipe.write(data.as_bytes());
                }
            },
            KeyCode::ArrowKey(arrow_key) => {
                let data = on_arrow_key(arrow_key, self.cursor_key_input_mode, self.modifier_key);
                let _ = self.write_pipe.write(data);
            },
            KeyCode::FunctionKey(function_key) => {
                let data = on_function_key(function_key);
                let _ = self.write_pipe.write(data);
            },
            KeyCode::ModifierKey(key) => {
                self.modifier_key.insert(key);
            },
        }
    }

    pub fn on_key_release(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::ModifierKey(key) => {
                self.modifier_key.remove(key);
            },
            _ => {},
        }
    }
}
