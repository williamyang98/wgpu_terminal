#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum KeyCode {
    UpArrow,
    DownArrow,
    RightArrow,
    LeftArrow,
    Home,
    End,
    Backspace,
    Pause,
    Escape,
    Insert,
    Delete,
    PageUp,
    PageDown,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum KeyModifier {
    None,
    Ctrl,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct KeyInput {
    pub modifier: KeyModifier,
    pub code: KeyCode,
}

impl KeyInput {
    pub(crate) fn new_simple(code: KeyCode) -> Self {
        Self {
            modifier: KeyModifier::None,
            code,
        }
    }
}

