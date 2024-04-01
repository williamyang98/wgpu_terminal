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

bitflags! {
    #[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
    pub struct ModifierKey: u8 {
        const None  = 0b0000_0000;
        const Ctrl  = 0b0000_0001;
        const Shift = 0b0000_0010;
        const Alt   = 0b0000_0100;
        const Meta  = 0b0000_1000;
        const _ = 0u8;
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

#[derive(Clone,Copy,Default,Debug,PartialEq,Eq)]
pub enum MouseTrackingMode {
    #[default]
    Disabled,
    X10,
    Normal,
    Highlight,
    Motion,
    Any,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum MouseCoordinateFormat {
    // https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
    // https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Extended-coordinates
    X10,
    Utf8,
    Sgr,
    Urxvt,
    SgrPixel,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum MouseButton {
    LeftClick,
    RightClick,
    MiddleClick,
    WheelUp,
    WheelDown,
    WheelLeft,
    WheelRight,
}

// get the position in pixels within the terminal window
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum MouseEvent {
    ButtonPress(MouseButton, Vector2<usize>),
    ButtonRelease(MouseButton, Vector2<usize>),
}

bitflags! {
    #[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
    struct ActiveMouseButtons: u8 {
        const None        = 0b0000_0001;
        const LeftClick   = 0b0000_0001;
        const RightClick  = 0b0000_0010;
        const MiddleClick = 0b0000_0100;
        const WheelUp     = 0b0000_1000;
        const WheelDown   = 0b0001_0000;
        const WheelLeft   = 0b0010_0000;
        const WheelRight  = 0b0100_0000;
        const _ = 0u8;
    }
}

fn mouse_button_to_flag(button: MouseButton) -> ActiveMouseButtons {
    match button {
        MouseButton::LeftClick => ActiveMouseButtons::LeftClick,
        MouseButton::RightClick => ActiveMouseButtons::RightClick,
        MouseButton::MiddleClick => ActiveMouseButtons::MiddleClick,
        MouseButton::WheelUp => ActiveMouseButtons::WheelUp,
        MouseButton::WheelDown => ActiveMouseButtons::WheelDown,
        MouseButton::WheelLeft => ActiveMouseButtons::WheelLeft,
        MouseButton::WheelRight => ActiveMouseButtons::WheelRight,
    }
}

pub struct Encoder {
    pub modifier_key: ModifierKey,
    pub keypad_input_mode: InputMode,
    pub cursor_key_input_mode: InputMode,
    pub mouse_tracking_mode: MouseTrackingMode,
    pub mouse_coordinate_format: MouseCoordinateFormat,
    pub window_size: Vector2<usize>,
    pub grid_size: Vector2<usize>,
    pub is_bracketed_paste_mode: bool,
    pub is_report_focus: bool,
    active_mouse_buttons: ActiveMouseButtons,
    utf8_encode_buffer: [u8;4],
    encode_buffer: Vec<u8>,
}

impl Default for Encoder {
    fn default() -> Self {
        Self {
            modifier_key: ModifierKey::None,
            keypad_input_mode: InputMode::Numeric,
            cursor_key_input_mode: InputMode::Numeric,
            mouse_tracking_mode: MouseTrackingMode::Disabled,
            mouse_coordinate_format: MouseCoordinateFormat::X10,
            window_size: Vector2::new(1,1),
            grid_size: Vector2::new(1,1),
            is_bracketed_paste_mode: false,
            is_report_focus: false,
            active_mouse_buttons: ActiveMouseButtons::None,
            utf8_encode_buffer: [0u8; 4],
            encode_buffer: Vec::with_capacity(256),
        }
    }
}

impl Encoder {
    pub fn on_key_press(&mut self, key_code: KeyCode, output: &mut impl FnMut(&[u8])) {
        match key_code {
            KeyCode::Char(c) => self.on_character(c, output),
            KeyCode::ArrowKey(arrow_key) => self.on_arrow_key(arrow_key, output),
            KeyCode::FunctionKey(function_key) => self.on_function_key(function_key, output),
            KeyCode::ModifierKey(modifier_key) => self.modifier_key.insert(modifier_key),
        }
    }

    pub fn on_key_release(&mut self, key_code: KeyCode, _output: &mut impl FnMut(&[u8])) {
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

    pub fn on_mouse_event(&mut self, event: MouseEvent, output: &mut impl FnMut(&[u8])) {
        // https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Button-event-tracking
        // https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Extended-coordinates
        // https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
        // (1,1) is the top,left corner
        self.encode_buffer.clear();
        match event {
            MouseEvent::ButtonPress(button, _) => self.active_mouse_buttons.insert(mouse_button_to_flag(button)),
            MouseEvent::ButtonRelease(button, _) => self.active_mouse_buttons.remove(mouse_button_to_flag(button)),
        }
        match self.mouse_tracking_mode {
            MouseTrackingMode::Disabled => {},
            MouseTrackingMode::X10 => {
                if let MouseEvent::ButtonPress(button, position) = event {
                    let event_code: u8 = match button {
                        // https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-X10-compatibility-mode
                        MouseButton::LeftClick   => 0,
                        MouseButton::RightClick  => 1,
                        MouseButton::MiddleClick => 2,
                        // https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Wheel-mice
                        MouseButton::WheelUp     => 64,
                        MouseButton::WheelDown   => 64+1,
                        MouseButton::WheelLeft   => 64+2,
                        MouseButton::WheelRight  => 64+3,
                    };
                    self.encode_buffer.extend_from_slice(b"\x1b[M");
                    self.encode_buffer.push(event_code);
                    let format = match self.mouse_coordinate_format {
                        MouseCoordinateFormat::Utf8 => MouseCoordinateFormat::Utf8,
                        _ => MouseCoordinateFormat::X10,
                    };
                    self.encode_mouse_position(position, format);
                    output(self.encode_buffer.as_slice());
                }
            },
            MouseTrackingMode::Normal | MouseTrackingMode::Motion | MouseTrackingMode::Any => {
                // let report_motion_event =
                //     (self.mouse_tracking_mode == MouseTrackingMode::Motion && !self.active_mouse_buttons.is_empty()) ||
                //     (self.mouse_tracking_mode == MouseTrackingMode::Any);
                let report_motion_event = false;
                let (mut button_event_code, is_pressed, position) = match event {
                    MouseEvent::ButtonPress(button, position) => {
                        let mut data = 0u8;
                        match button {
                            MouseButton::LeftClick   => { data |= 0b0000_0000; },
                            MouseButton::RightClick  => { data |= 0b0000_0001; },
                            MouseButton::MiddleClick => { data |= 0b0000_0010; },
                            _ => return, // no encoding for this
                        }
                        (data, true, position)
                    },
                    MouseEvent::ButtonRelease(_, position) => {
                        let data = 0b0000_0011;
                        (data, false, position)
                    },
                };
                if self.modifier_key.contains(ModifierKey::Shift) { button_event_code |= 0b0000_0100; }
                if self.modifier_key.contains(ModifierKey::Meta)  { button_event_code |= 0b0000_1000; }
                if self.modifier_key.contains(ModifierKey::Ctrl)  { button_event_code |= 0b0001_0000; }
                if report_motion_event { 
                    button_event_code |= 0b0010_0000; 
                }
                match self.mouse_coordinate_format {
                    MouseCoordinateFormat::X10 | MouseCoordinateFormat::Utf8 => {
                        self.encode_buffer.extend_from_slice(b"\x1b[M");
                        button_event_code += 32; // used to make sure that it is an ascii character
                        self.encode_buffer.push(button_event_code);
                        self.encode_mouse_position(position, self.mouse_coordinate_format);
                        output(self.encode_buffer.as_slice());
                    },
                    MouseCoordinateFormat::Sgr | MouseCoordinateFormat::SgrPixel => {
                        self.encode_buffer.extend_from_slice(b"\x1b[<");
                        let _ = write!(&mut self.encode_buffer, "{}", button_event_code);
                        self.encode_buffer.push(b';');
                        self.encode_mouse_position(position, self.mouse_coordinate_format);
                        if is_pressed {
                            self.encode_buffer.push(b'M');
                        } else {
                            self.encode_buffer.push(b'm');
                        }
                        output(self.encode_buffer.as_slice());
                    },
                    MouseCoordinateFormat::Urxvt => {
                        self.encode_buffer.extend_from_slice(b"\x1b[");
                        let _ = write!(&mut self.encode_buffer, "{}", button_event_code);
                        self.encode_buffer.push(b';');
                        self.encode_mouse_position(position, self.mouse_coordinate_format);
                        self.encode_buffer.push(b'M');
                        output(self.encode_buffer.as_slice());
                    },
                }
            },
            MouseTrackingMode::Highlight => {
                // TODO: highlight tracking (this seems very complicated for just highlight text???)
                // if button press or release generate normal mode events
                // Warning: this mode requires a cooperating program, else xterm will hang.
                //          the program should respond with ESC[#;#;#;#;#T where 
                //          #1: 0 = exit highlight, >0 = start highlight
                //          #2: start.x
                //          #3: start.y
                //          #4: start_row
                //          #5: end_row
                // let mut is_highlight_tracking = true;
                // let mut is_mouse_left_click = true;
                // let mut start_highlight_position = None;
                // let mut last_highlight_position = None;
                // if let MouseEvent::Move(position) = event {
                //     if start_highlight_position.is_none() {
                //         start_highlight_position = position;
                //     }
                // }
            }
        }
    }

    fn encode_mouse_position(&mut self, pos: Vector2<usize>, format: MouseCoordinateFormat) {
        // https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Extended-coordinates
        let glyph_size = Vector2::new(
            self.window_size.x.div_ceil(self.grid_size.x.max(1)).max(1),
            self.window_size.y.div_ceil(self.grid_size.y.max(1)).max(1),
        );
        // (1,1) is the origin point
        let grid_pos = Vector2::new(
            (pos.x/glyph_size.x)+1,
            (pos.y/glyph_size.y)+1,
        );
        match format {
            MouseCoordinateFormat::X10 => {
                // x10 adds 32 to everything so that it is within ascii range for some reason
                self.encode_buffer.push((grid_pos.x+32).min(255) as u8);
                self.encode_buffer.push((grid_pos.y+32).min(255) as u8);
            },
            MouseCoordinateFormat::Utf8 => {
                
            },
            MouseCoordinateFormat::Sgr | MouseCoordinateFormat::Urxvt => {
                let _ = write!(&mut self.encode_buffer, "{};{}", grid_pos.x, grid_pos.y);
            },
            MouseCoordinateFormat::SgrPixel => {
                let _ = write!(&mut self.encode_buffer, "{};{}", pos.x, pos.y);
            },
        }
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

    pub fn set_window_size_characters(&mut self, size: Vector2<usize>, output: &mut impl FnMut(&[u8])) {
        // https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Miscellaneous
        self.encode_buffer.clear();
        if write!(&mut self.encode_buffer, "\x1b[18;{};{}t", size.x, size.y).is_ok() {
            output(self.encode_buffer.as_slice());
        }
    }

    pub fn paste_text(&mut self, buf: &[u8], output: &mut impl FnMut(&[u8])) {
        // https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Bracketed-Paste-Mode
        if self.is_bracketed_paste_mode {
            output(b"\x1b[200~"); 
            output(buf);
            output(b"\x1b[201~"); 
        } else {
            output(buf);
        }
    }
}
