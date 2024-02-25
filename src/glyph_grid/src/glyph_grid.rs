use bytemuck::{Pod, Zeroable};
use cgmath::{Vector2, Vector4};
use crate::view_2d::{MutableView2d, ImmutableView2d};

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub struct GlyphGridData {
    data: [u32; 4],
}

impl GlyphGridData {
    pub fn set_page_index(&mut self, page_index: usize) {
        self.data[0] = (self.data[0] & 0xFFFFFF00) | ((page_index as u32) & 0xFF);
    }
    pub fn set_glyph_position(&mut self, pos: Vector2<usize>) {
        let pos = pos.cast::<u32>().unwrap();
        self.data[0] = 
             (self.data[0] & 0x000000FF) |
            ((pos.x << 8)  & 0x000FFF00) |
            ((pos.y << 20) & 0xFFF00000);
    }
    pub fn set_foreground_colour(&mut self, colour: Vector4<u8>) {
        let colour = colour.cast::<u32>().unwrap();
        self.data[1] = colour.x  | (colour.y << 8) | (colour.z << 16) | (colour.w << 24);
    }
    pub fn set_background_colour(&mut self, colour: Vector4<u8>) {
        let colour = colour.cast::<u32>().unwrap();
        self.data[2] = colour.x  | (colour.y << 8) | (colour.z << 16) | (colour.w << 24);
    }
    pub fn set_is_underline(&mut self, is_underline: bool) {
        let bit: u32 = if is_underline { 0b1 } else { 0b0 };
        self.data[3] = (self.data[3] & !0b1u32) | bit;
    }
}

pub struct GlyphGrid {
    data: Vec<GlyphGridData>,
    size: Vector2<usize>,
}

impl GlyphGrid {
    pub fn new(size: Vector2<usize>) -> Self {
        let mut data = Vec::<GlyphGridData>::new();
        data.resize(size.x*size.y, GlyphGridData::zeroed());
        Self {
            data,
            size,
        }
    }

    pub fn get_size(&self) -> Vector2<usize> {
        self.size
    }

    pub fn get_view(&self) -> ImmutableView2d<'_, GlyphGridData> {
        ImmutableView2d {
            data: self.data.as_slice(),
            row_stride: self.size.x,
            size: self.size,
        }
    }

    pub fn get_mut_view(&mut self) -> MutableView2d<'_, GlyphGridData> {
        MutableView2d {
            data: self.data.as_mut_slice(),
            row_stride: self.size.x,
            size: self.size,
        }
    }

    pub fn resize(&mut self, size: Vector2<usize>) {
        assert!(size.x >= 1);
        assert!(size.y >= 1);
        self.data.resize(size.x*size.y, GlyphGridData::zeroed());
        self.size = size;
    }
}

