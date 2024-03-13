use cgmath::Vector2;

pub trait GlyphGenerator {
    fn get_glyph_size(&self) -> Vector2<usize>;
    fn generate_glyph(&mut self, character: char) -> &[u8];
    fn has_glyph(&self, character: char) -> bool;
}

#[derive(Clone,Debug)]
pub struct FontdueGlyphGenerator {
    font: fontdue::Font,
    font_size_em: f32,
    glyph_baseline: usize,
    glyph_size: Vector2<usize>,
    temp_glyph_buffer: Vec<u8>,
}

impl FontdueGlyphGenerator {
    pub fn new(font: fontdue::Font, font_size_em: f32) -> Self {
        let font_line_metrics = font.horizontal_line_metrics(font_size_em).expect("Horizontal font expected");
        let glyph_baseline = font_line_metrics.ascent as usize;
        let glyph_height = (font_line_metrics.ascent - font_line_metrics.descent) as usize;
        let glyph_width = font.metrics(' ', font_size_em).advance_width as usize;
        let glyph_size = Vector2::<usize>::new(glyph_width, glyph_height);
        Self {
            font,
            font_size_em,
            glyph_baseline,
            glyph_size,
            temp_glyph_buffer: vec![0u8; glyph_size.x*glyph_size.y],
        }
    }
}

impl GlyphGenerator for FontdueGlyphGenerator {
    fn get_glyph_size(&self) -> Vector2<usize> {
        self.glyph_size
    }

    fn generate_glyph(&mut self, character: char) -> &[u8] {
        let (metrics, bitmap) = self.font.rasterize(character, self.font_size_em);
        // determine position of glyph from baseline
        let y_offset = self.glyph_baseline as i32 - metrics.ymin - metrics.height as i32;
        let x_offset = metrics.xmin;
        // If glyph overflows or underflows view we shifted it and clamp its location and size
        let y_offset = y_offset.clamp(0, self.glyph_size.y as i32) as usize;
        let x_offset = x_offset.clamp(0, self.glyph_size.x as i32) as usize;
        let height = metrics.height.min(self.glyph_size.y-y_offset);
        let width = metrics.width.min(self.glyph_size.x-x_offset);

        self.temp_glyph_buffer.fill(0u8);
        assert!((x_offset + width) <= self.glyph_size.x);
        assert!((y_offset + height) <= self.glyph_size.y);
        for y in 0..height {
            let i_src = y*metrics.width;
            let i_dst = (y+y_offset)*self.glyph_size.x + x_offset;
            let row_src = &bitmap[i_src..(i_src+width)];
            let row_dst = &mut self.temp_glyph_buffer[i_dst..(i_dst+width)];
            row_dst.copy_from_slice(row_src);
        }
        self.temp_glyph_buffer.as_slice()
    }

    fn has_glyph(&self, character: char) -> bool {
        self.font.has_glyph(character)
    }
}
