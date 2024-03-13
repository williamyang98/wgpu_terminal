use crate::lru_list::LruList;
use crate::glyph_atlas::{GlyphAtlas,GlyphIndex};
use crate::glyph_generator::GlyphGenerator;
use std::collections::HashMap;

#[derive(Clone,Copy,Debug)]
struct GlyphEntry {
    character: char,
    atlas_index: GlyphIndex,
    render_id: usize,
}

// Glyph atlas texture will be divided into blocks to avoid many sparse small texture uploads
pub struct GlyphCache {
    glyph_generator: Box<dyn GlyphGenerator>,
    glyph_atlas: GlyphAtlas,
    ascii_atlas_index: Vec<GlyphIndex>,
    lru_glyph_index: HashMap<char,usize>,
    lru_glyph_list: LruList<GlyphEntry>,
}

const ASCII_GLYPH_START: char = ' ';
const ASCII_GLYPH_END: char = '~';
const ASCII_TOTAL_GLYPHS: usize = ASCII_GLYPH_END as usize - ASCII_GLYPH_START as usize + 2;

impl GlyphCache {
    pub fn new(glyph_generator: Box<dyn GlyphGenerator>) -> Self {
        let glyph_size = glyph_generator.get_glyph_size();
        let glyph_atlas = GlyphAtlas::new(glyph_size);
 
        let mut cache = Self {
            glyph_generator,
            glyph_atlas,
            ascii_atlas_index: Vec::with_capacity(ASCII_TOTAL_GLYPHS),
            lru_glyph_index: HashMap::new(),
            lru_glyph_list: LruList::default(),
        };
        cache.generate_ascii_glyphs();
        cache
    }

    fn generate_ascii_glyphs(&mut self) {
        for c in ASCII_GLYPH_START..=ASCII_GLYPH_END {
            let atlas_index = self.glyph_atlas.get_new_free_index();
            let atlas_index = atlas_index.expect("Glyph atlas should have enough room for ascii characters");
            let glyph_data = self.glyph_generator.generate_glyph(c);
            self.glyph_atlas.write_glyph(atlas_index, glyph_data);
            self.ascii_atlas_index.push(atlas_index);
        }
    }

    fn get_ascii_atlas_index(&self, c: char) -> GlyphIndex {
        let glyph_index = (c as usize) - (ASCII_GLYPH_START as usize);
        self.ascii_atlas_index[glyph_index]
    }

    pub fn get_glyph_atlas(&self) -> &GlyphAtlas {
        &self.glyph_atlas
    }

    pub fn get_glyph_atlas_mut(&mut self) -> &mut GlyphAtlas {
        &mut self.glyph_atlas
    }

    pub fn get_glyph_location(&mut self, c: char, render_id: usize) -> GlyphIndex {
        if (ASCII_GLYPH_START..=ASCII_GLYPH_END).contains(&c) {
            return self.get_ascii_atlas_index(c);
        }

        // fallback to default glyph if not in atlas
        let c = if self.glyph_generator.has_glyph(c) { c } else { '\0' };
        // check if glyph in cache
        if let Some(glyph_index) = self.lru_glyph_index.get(&c) {
            let glyph_entry = self.lru_glyph_list.get_mut_data(*glyph_index);
            glyph_entry.render_id = render_id;
            return glyph_entry.atlas_index;
        }
        let atlas_index = match self.glyph_atlas.get_new_free_index() {
            Some(atlas_index) => {
                let glyph_index = self.lru_glyph_list.push(&GlyphEntry { 
                    character: c, 
                    atlas_index,
                    render_id,
                });
                self.lru_glyph_index.insert(c, glyph_index);
                atlas_index
            },
            None => {
                let glyph_index = self.lru_glyph_list.get_oldest().expect("Glyph cache should already be populated");
                let glyph_entry = self.lru_glyph_list.get_mut_data(glyph_index);
                let old_character = glyph_entry.character;
                let atlas_index = glyph_entry.atlas_index;
                if glyph_entry.render_id == render_id {
                    log::warn!("evicting glyph that is still in use '{}' with '{}' at render_id={}", 
                        old_character, c, render_id);
                }
                glyph_entry.character = c;
                glyph_entry.render_id = render_id;
                let _is_promoted = self.lru_glyph_list.promote(glyph_index);
                self.lru_glyph_index.remove(&old_character);
                self.lru_glyph_index.insert(c, glyph_index);
                atlas_index
            },
        };

        let glyph_data = self.glyph_generator.generate_glyph(c);
        self.glyph_atlas.write_glyph(atlas_index, glyph_data);
        atlas_index
    }
}
