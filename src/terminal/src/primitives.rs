use bitflags::bitflags;
use vt100::common::Rgb8;

bitflags! {
    #[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
    pub struct StyleFlags: u8 {
        const None          = 0b0000_0000;
        const Bold          = 0b0000_0001;
        const Dim           = 0b0000_0010;
        const Italic        = 0b0000_0100;
        const Underline     = 0b0000_1000;
        const Blinking      = 0b0001_0000;
        const Inverse       = 0b0010_0000;
        const Hidden        = 0b0100_0000;
        const Strikethrough = 0b1000_0000;
        const _ = 0u8;
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Cell {
    pub character: char, // 4
    pub background_colour: Rgb8, // 3
    pub foreground_colour: Rgb8, // 3
    pub style_flags: StyleFlags, // 1
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            character: ' ',
            background_colour: Rgb8 { r:0, g:0, b:0 },
            foreground_colour: Rgb8 { r:255, g:255, b:255 },
            style_flags: StyleFlags::None,
        }
    }
}

#[derive(Clone,Copy,Default,Debug,PartialEq,Eq)]
pub struct Pen {
    pub background_colour: Rgb8,
    pub foreground_colour: Rgb8,
    pub style_flags: StyleFlags,
}

impl Pen {
    #[inline]
    pub fn colour_in_cell(&self, cell: &mut Cell) {
        cell.background_colour = self.background_colour;
        cell.foreground_colour = self.foreground_colour;
        cell.style_flags = self.style_flags;
    }
}
