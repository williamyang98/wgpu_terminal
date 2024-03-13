use cgmath::Vector2;

#[derive(Clone,Copy,Debug)]
pub struct GlyphIndex {
    pub block: Vector2<usize>,
    pub position: Vector2<usize>, 
}

impl Default for GlyphIndex {
    fn default() -> Self {
        Self {
            block: Vector2::new(0,0),
            position: Vector2::new(0,0),
        }
    }
}

#[derive(Clone)]
pub struct GlyphAtlas {
    data: Vec<u8>,
    glyph_size: Vector2<usize>,
    total_glyphs_in_block: Vector2<usize>,
    total_blocks: Vector2<usize>,
    max_blocks: Vector2<usize>,
    free_index: GlyphIndex,
    total_modified_glyphs_per_block: Vec<usize>,
}

impl GlyphAtlas {
    pub(crate) fn new(glyph_size: Vector2<usize>, max_texture_size: Vector2<usize>) -> Self {
        // determine best block size
        let max_grid_size = Vector2::new(
            max_texture_size.x / glyph_size.x,
            max_texture_size.y / glyph_size.y,
        );
        const DESIRED_GLYPHS_IN_BLOCK: Vector2<usize> = Vector2::new(16,8);
        let max_blocks = Vector2::new(
            (max_grid_size.x/DESIRED_GLYPHS_IN_BLOCK.x).max(1),
            (max_grid_size.y/DESIRED_GLYPHS_IN_BLOCK.y).max(1),
        );
        let total_glyphs_in_block = Vector2::new(
            max_grid_size.x/max_blocks.x,
            max_grid_size.y/max_blocks.y,
        );
        log::info!(
            "Creating glyph atlas with: glyph_size=({},{}) glyphs_in_block=({},{}) max_blocks=({},{}) max_size=({},{})",
            glyph_size.x, glyph_size.y,
            total_glyphs_in_block.x, total_glyphs_in_block.y,
            max_blocks.x, max_blocks.y,
            glyph_size.x*total_glyphs_in_block.x*max_blocks.x, glyph_size.y*total_glyphs_in_block.y*max_blocks.y,
        );
        let mut atlas = Self {
            data: Vec::new(),
            glyph_size,
            total_glyphs_in_block,
            total_blocks: Vector2::new(0,0),
            max_blocks,
            free_index: GlyphIndex::default(),
            total_modified_glyphs_per_block: Vec::new(),
        };
        atlas.resize(Vector2::new(1,1));
        atlas
    }

    fn resize(&mut self, total_blocks: Vector2<usize>) {
        let length = total_blocks.x*total_blocks.y;
        self.total_blocks = total_blocks;
        self.total_modified_glyphs_per_block.resize(length, 0);
        self.total_modified_glyphs_per_block.fill(1);
        let texture_size = self.get_texture_size();
        self.data.resize(texture_size.x*texture_size.y, 0u8);
        log::info!("Resizing glyph atlas to {}x{}", total_blocks.x, total_blocks.y);
    }

    pub fn get_block(&self, block: Vector2<usize>) -> &[u8] {
        assert!(block.x < self.total_blocks.x);
        assert!(block.y < self.total_blocks.y);
        let glyph_stride = self.glyph_size.x*self.glyph_size.y;
        let total_glyphs_in_block = self.total_glyphs_in_block.x*self.total_glyphs_in_block.y;
        let block_stride = total_glyphs_in_block*glyph_stride;
        let block_index = block.x + block.y*self.total_blocks.x;
        let block_offset = block_index*block_stride;
        &self.data[block_offset..(block_offset+block_stride)]
    }

    pub(crate) fn write_glyph(&mut self, index: GlyphIndex, data: &[u8]) {
        assert!(index.block.x < self.total_blocks.x);
        assert!(index.block.y < self.total_blocks.y);
        assert!(index.position.x < self.total_glyphs_in_block.x);
        assert!(index.position.y < self.total_glyphs_in_block.y);
        assert!(data.len() == (self.glyph_size.y*self.glyph_size.x));

        let glyph_stride = self.glyph_size.x*self.glyph_size.y;
        let total_glyphs_in_block = self.total_glyphs_in_block.x*self.total_glyphs_in_block.y;
        let dst_block = {
            let block_stride = total_glyphs_in_block*glyph_stride;
            let block_index = index.block.x + index.block.y*self.total_blocks.x;
            let block_offset = block_index*block_stride;
            &mut self.data[block_offset..(block_offset+block_stride)]
        };
 
        let row_stride = self.total_glyphs_in_block.x*self.glyph_size.x;
        let glyph_offset = 
            index.position.x*self.glyph_size.x +
            index.position.y*self.glyph_size.y*row_stride;

        for y in 0..self.glyph_size.y {
            let i_dst = glyph_offset + y*row_stride;
            let i_src = y*self.glyph_size.x;
            let src_buf = &data[i_src..(i_src+self.glyph_size.x)];
            let dst_buf = &mut dst_block[i_dst..(i_dst+self.glyph_size.x)];
            dst_buf.copy_from_slice(src_buf);
        }
        let block_index = index.block.y*self.total_blocks.x + index.block.x;
        self.total_modified_glyphs_per_block[block_index] += 1;
    }

    pub(crate) fn get_free_index(&mut self) -> Option<GlyphIndex> {
        if self.free_index.block.y >= self.max_blocks.y {
            return None;
        }
        assert!(self.free_index.block.x < self.max_blocks.x);
        // resize to fit free index
        let new_total_blocks = Vector2::new(
            self.total_blocks.x.max(self.free_index.block.x+1),
            self.total_blocks.y.max(self.free_index.block.y+1),
        );
        if new_total_blocks != self.total_blocks {
            self.resize(new_total_blocks);
        }
        Some(self.free_index)
    }

    pub(crate) fn increment_free_index(&mut self) -> bool {
        if self.free_index.block.y >= self.max_blocks.y {
            return false;
        }
        self.free_index.position.x += 1;
        if self.free_index.position.x >= self.total_glyphs_in_block.x {
            self.free_index.position.x = 0;
            self.free_index.position.y += 1;
        }
        if self.free_index.position.y >= self.total_glyphs_in_block.y {
            self.free_index.position = Vector2::new(0,0);
            self.free_index.block.x += 1;
        }
        if self.free_index.block.x >= self.max_blocks.x {
            self.free_index.block.x = 0;
            self.free_index.block.y += 1;
        }
        true
    }

    pub fn get_modified_blocks(&self) -> impl Iterator<Item=Vector2<usize>> + '_ {
        self.total_modified_glyphs_per_block
            .iter()
            .enumerate()
            .filter(|(_i,count)| {
                **count > 0usize
            })
            .map(|(i,_count)| {
                let row = i / self.total_blocks.x;
                let col = i % self.total_blocks.x;
                Vector2::<usize>::new(col, row)
            })
    }

    pub fn clear_modified_count(&mut self) {
        self.total_modified_glyphs_per_block.fill(0);
    }

    pub fn get_glyph_size(&self) -> Vector2<usize> {
        self.glyph_size
    }

    pub fn get_total_glyphs_in_block(&self) -> Vector2<usize> {
        self.total_glyphs_in_block
    }

    pub fn get_total_blocks(&self) -> Vector2<usize> {
        self.total_blocks
    }

    pub fn get_texture_size(&self) -> Vector2<usize> {
        Vector2::new(
            self.glyph_size.x*self.total_glyphs_in_block.x*self.total_blocks.x,
            self.glyph_size.y*self.total_glyphs_in_block.y*self.total_blocks.y,
        )
    }
}

