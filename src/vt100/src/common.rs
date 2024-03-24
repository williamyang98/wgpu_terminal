use cgmath::Vector2;

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

#[derive(Clone,Debug,PartialEq,Eq)]
pub enum WindowAction {
    Move(Vector2<u16>),
    Resize(Vector2<u16>),
    SendToFront,
    SendToBack,
    Refresh,
    ResizeTextArea(Vector2<u16>),
    RestoreMaximised,
    Maximise(Vector2<bool>),
    SetWindowTitle(String),
    SetIconTitle(String),
    SetFullscreen(bool),
    SetMinimised(bool),
    ToggleFullscreen,
    SaveIconTitle(Option<u16>),
    SaveWindowTitle(Option<u16>),
    RestoreIconTitle(Option<u16>),
    RestoreWindowTitle(Option<u16>),
    ResizeWindowHeight(u16),
    GetWindowState,
    GetWindowPosition,
    GetTextAreaPosition,
    GetTextAreaSize,
    GetWindowSize,
    GetScreenSize,
    GetCellSize,
    GetTextAreaGridSize,
    GetScreenGridSize,
    GetWindowIconLabel,
    GetWindowTitle,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum BellVolume {
    Off,
    Low,
    High,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ColourMode {
    Monochrome,
    Colour,
    Colour2bit,
    Colour4bit,
    Colour8bit,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum GraphicsMode {
    Text,
    Graphics,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct ScreenMode {
    pub size: Vector2<u16>,
    pub colour_mode: ColourMode,
    pub graphics_mode: GraphicsMode,
}

impl ScreenMode {
    pub(crate) fn try_from_u16(code: u16) -> Option<Self> {
        // https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797#screen-modes
        match code {
            0 => Some(ScreenMode {
                size: Vector2::new(40, 25), 
                colour_mode: ColourMode::Monochrome,
                graphics_mode: GraphicsMode::Text,
            }),
            1 => Some(ScreenMode {
                size: Vector2::new(40, 25), 
                colour_mode: ColourMode::Colour,
                graphics_mode: GraphicsMode::Text,
            }),
            2 => Some(ScreenMode {
                size: Vector2::new(80, 25), 
                colour_mode: ColourMode::Monochrome,
                graphics_mode: GraphicsMode::Text,
            }),
            3 => Some(ScreenMode {
                size: Vector2::new(80, 25), 
                colour_mode: ColourMode::Colour,
                graphics_mode: GraphicsMode::Text,
            }),
            4 => Some(ScreenMode {
                size: Vector2::new(320, 200), 
                colour_mode: ColourMode::Colour2bit,
                graphics_mode: GraphicsMode::Graphics,
            }),
            5 => Some(ScreenMode {
                size: Vector2::new(320, 200), 
                colour_mode: ColourMode::Monochrome,
                graphics_mode: GraphicsMode::Graphics,
            }),
            6 => Some(ScreenMode {
                size: Vector2::new(640, 200), 
                colour_mode: ColourMode::Monochrome,
                graphics_mode: GraphicsMode::Graphics,
            }),
            13 => Some(ScreenMode {
                size: Vector2::new(320, 200), 
                colour_mode: ColourMode::Colour,
                graphics_mode: GraphicsMode::Graphics,
            }),
            14 => Some(ScreenMode {
                size: Vector2::new(640, 200), 
                colour_mode: ColourMode::Colour4bit,
                graphics_mode: GraphicsMode::Graphics,
            }),
            15 => Some(ScreenMode {
                size: Vector2::new(640, 350), 
                colour_mode: ColourMode::Monochrome,
                graphics_mode: GraphicsMode::Graphics,
            }),
            16 => Some(ScreenMode {
                size: Vector2::new(640, 350), 
                colour_mode: ColourMode::Colour4bit,
                graphics_mode: GraphicsMode::Graphics,
            }),
            17 => Some(ScreenMode {
                size: Vector2::new(640, 480), 
                colour_mode: ColourMode::Monochrome,
                graphics_mode: GraphicsMode::Graphics,
            }),
            18 => Some(ScreenMode {
                size: Vector2::new(640, 480), 
                colour_mode: ColourMode::Colour4bit,
                graphics_mode: GraphicsMode::Graphics,
            }),
            19 => Some(ScreenMode {
                size: Vector2::new(320, 200), 
                colour_mode: ColourMode::Colour8bit,
                graphics_mode: GraphicsMode::Graphics,
            }),
            _ => None,
        }
    }
}

#[derive(Clone,Copy,Default,Debug,PartialEq,Eq)]
pub struct Rgb8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum GraphicStyle {
    ResetAll,
    EnableBold,
    EnableDim,
    EnableItalic,
    EnableUnderline,
    EnableBlinking,
    EnableInverse,
    EnableHidden,
    EnableStrikethrough,
    DisableWeight,
    DisableItalic,
    DisableUnderline,
    DisableBlinking,
    DisableInverse,
    DisableHidden,
    DisableStrikethrough,
    ForegroundBlack,
    ForegroundRed,
    ForegroundGreen,
    ForegroundYellow,
    ForegroundBlue,
    ForegroundMagenta,
    ForegroundCyan,
    ForegroundWhite,
    ForegroundExtended,
    ForegroundDefault,
    BackgroundBlack,
    BackgroundRed,
    BackgroundGreen,
    BackgroundYellow,
    BackgroundBlue,
    BackgroundMagenta,
    BackgroundCyan,
    BackgroundWhite,
    BackgroundExtended,
    BackgroundDefault,
    BrightForegroundBlack,
    BrightForegroundRed,
    BrightForegroundGreen,
    BrightForegroundYellow,
    BrightForegroundBlue,
    BrightForegroundMagenta,
    BrightForegroundCyan,
    BrightForegroundWhite,
    BrightBackgroundBlack,
    BrightBackgroundRed,
    BrightBackgroundGreen,
    BrightBackgroundYellow,
    BrightBackgroundBlue,
    BrightBackgroundMagenta,
    BrightBackgroundCyan,
    BrightBackgroundWhite,
}

impl GraphicStyle {
    pub(crate) fn try_from_u16(v: u16) -> Option<Self> {
        match v {
              0 => Some(Self::ResetAll),
              1 => Some(Self::EnableBold),
              2 => Some(Self::EnableDim),
              3 => Some(Self::EnableItalic),
              4 => Some(Self::EnableUnderline),
              5 => Some(Self::EnableBlinking),
              7 => Some(Self::EnableInverse),
              8 => Some(Self::EnableHidden),
              9 => Some(Self::EnableStrikethrough),
             22 => Some(Self::DisableWeight),
             23 => Some(Self::DisableItalic),
             24 => Some(Self::DisableUnderline),
             25 => Some(Self::DisableBlinking),
             27 => Some(Self::DisableInverse),
             28 => Some(Self::DisableHidden),
             29 => Some(Self::DisableStrikethrough),
             30 => Some(Self::ForegroundBlack),
             31 => Some(Self::ForegroundRed),
             32 => Some(Self::ForegroundGreen),
             33 => Some(Self::ForegroundYellow),
             34 => Some(Self::ForegroundBlue),
             35 => Some(Self::ForegroundMagenta),
             36 => Some(Self::ForegroundCyan),
             37 => Some(Self::ForegroundWhite),
             38 => Some(Self::ForegroundExtended),
             39 => Some(Self::ForegroundDefault),
             40 => Some(Self::BackgroundBlack),
             41 => Some(Self::BackgroundRed),
             42 => Some(Self::BackgroundGreen),
             43 => Some(Self::BackgroundYellow),
             44 => Some(Self::BackgroundBlue),
             45 => Some(Self::BackgroundMagenta),
             46 => Some(Self::BackgroundCyan),
             47 => Some(Self::BackgroundWhite),
             48 => Some(Self::BackgroundExtended),
             49 => Some(Self::BackgroundDefault),
             90 => Some(Self::BrightForegroundBlack),
             91 => Some(Self::BrightForegroundRed),
             92 => Some(Self::BrightForegroundGreen),
             93 => Some(Self::BrightForegroundYellow),
             94 => Some(Self::BrightForegroundBlue),
             95 => Some(Self::BrightForegroundMagenta),
             96 => Some(Self::BrightForegroundCyan),
             97 => Some(Self::BrightForegroundWhite),
            100 => Some(Self::BrightBackgroundBlack),
            101 => Some(Self::BrightBackgroundRed),
            102 => Some(Self::BrightBackgroundGreen),
            103 => Some(Self::BrightBackgroundYellow),
            104 => Some(Self::BrightBackgroundBlue),
            105 => Some(Self::BrightBackgroundMagenta),
            106 => Some(Self::BrightBackgroundCyan),
            107 => Some(Self::BrightBackgroundWhite),
            _ => None,
        }
    }
}

