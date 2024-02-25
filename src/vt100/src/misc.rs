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
