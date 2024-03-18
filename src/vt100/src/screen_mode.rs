use crate::misc::Vector2;

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
