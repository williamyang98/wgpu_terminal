use bitflags::bitflags;
use vt100::graphic_style::Rgb8;

bitflags! {
    #[derive(Clone,Copy,Debug,Default)]
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

// pad to 16bytes so we can store them in circular scrollback buffer
#[repr(C)]
#[derive(Clone,Copy,Default,Debug)]
pub struct Cell {
    pub character: char, // 4
    pub background_colour: Rgb8, // 3
    pub foreground_colour: Rgb8, // 3
    pub style_flags: StyleFlags, // 1
}

#[derive(Clone,Copy,Default,Debug)]
pub struct Pen {
    pub background_colour: Rgb8,
    pub foreground_colour: Rgb8,
    pub style_flags: StyleFlags,
}

impl Cell {
    pub fn colour_from_pen(&mut self, pen: &Pen) {
        self.background_colour = pen.background_colour;
        self.foreground_colour = pen.foreground_colour;
        self.style_flags = pen.style_flags;
    }
}
