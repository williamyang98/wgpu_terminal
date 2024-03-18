#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Vector2<T> {
    pub x: T,
    pub y: T,
}

impl<T> Vector2<T> {
    pub fn new(x: T, y: T) -> Self
    where T: Copy 
    {
        Self { x, y }
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct ScrollRegion {
    pub top: u16,
    pub bottom: u16,
}

impl ScrollRegion {
    pub fn new(top: u16, bottom: u16) -> Self {
        Self { top, bottom }
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum EraseMode {
    FromCursorToEnd,
    FromCursorToStart,
    EntireDisplay,
    SavedLines,
}

impl EraseMode {
    pub(crate) fn try_from_u16(v: u16) -> Option<Self> {
        // https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797#erase-functions
        match v {
            0 => Some(EraseMode::FromCursorToEnd),
            1 => Some(EraseMode::FromCursorToStart),
            2 => Some(EraseMode::EntireDisplay),
            3 => Some(EraseMode::SavedLines),
            _ => None,
        }
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum CharacterSet {
    Ascii,
    LineDrawing,
}

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
pub enum WindowAction {
    Iconify(bool),
    Move(Vector2<u16>),
    Resize(Vector2<u16>),
    SendToFront,
    SendToBack,
    Refresh,
    ResizeTextArea(Vector2<u16>),
    RestoreMaximised,
    Maximise(Vector2<bool>),
    SetFullscreen(bool),
    ToggleFullscreen,
    ReportWindowState,
    ReportWindowPosition,
    ReportTextAreaPosition,
    ReportTextAreaSize,
    ReportWindowSize,
    ReportScreenSize,
    ReportCellSize,
    ReportTextAreaGridSize,
    ReportScreenGridSize,
    ReportWindowIconLabel,
    ReportWindowTitle,
    SaveIconTitle(Option<u16>),
    SaveWindowTitle(Option<u16>),
    RestoreIconTitle(Option<u16>),
    RestoreWindowTitle(Option<u16>),
    ResizeWindowHeight(u16),
}
