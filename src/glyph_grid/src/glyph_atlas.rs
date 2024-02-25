use cgmath::{Vector2,ElementWise};
use crate::view_2d::{ImmutableView2d,MutableView2d};

pub struct GlyphAtlas<T> {
    data: Vec<T>,
    glyph_size: Vector2<usize>,
    grid_size: Vector2<usize>,
    texture_size: Vector2<usize>,
}

impl<T> GlyphAtlas<T> {
    pub(crate) fn new(glyph_size: Vector2<usize>, grid_size: Vector2<usize>) -> Self
    where T: Default + Clone
    {
        let texture_size = grid_size.mul_element_wise(glyph_size);
        let initial_size = texture_size.x*texture_size.y;
        let mut data = Vec::<T>::with_capacity(initial_size);
        data.resize(initial_size, T::default());
        Self {
            data,
            glyph_size,
            grid_size,
            texture_size,
        }
    }

    pub(crate) fn get_mut_glyph_view(&mut self, position: Vector2<usize>) -> MutableView2d<'_, T> {
        assert!(position.x < self.grid_size.x);
        assert!(position.y < self.grid_size.y);
        let row_offset = self.texture_size.x*self.glyph_size.y*position.y;
        let col_offset = self.glyph_size.x*position.x;
        let offset = row_offset + col_offset;
        let max_read_length = self.texture_size.x*(self.glyph_size.y-1) + self.glyph_size.x;
        let offset_end = offset + max_read_length;
        MutableView2d {
            data: &mut self.data[offset..offset_end],
            row_stride: self.texture_size.x,
            size: self.glyph_size,
        }
    }

    pub fn get_texture_view(&self) -> ImmutableView2d<'_, T> {
        ImmutableView2d {
            data: self.data.as_slice(),
            row_stride: self.texture_size.x,
            size: self.texture_size,
        }
    }

    pub fn get_grid_size(&self) -> Vector2<usize> {
        self.grid_size
    }

    pub fn get_glyph_size(&self) -> Vector2<usize> {
        self.glyph_size
    }

    pub fn get_texture_size(&self) -> Vector2<usize> {
        self.texture_size
    }
}
