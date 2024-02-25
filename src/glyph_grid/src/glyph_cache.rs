use bytemuck::{Pod, Zeroable};
use cgmath::{Vector2,Zero};
use crate::{
    glyph_atlas::GlyphAtlas,
    view_2d::MutableView2d,
    lru_list::LruList,
};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct GlyphCacheLocation {
    pub page_index: usize,
    pub glyph_position: Vector2<usize>,
}

impl GlyphCacheLocation {
    fn zero() -> Self {
        Self {
            page_index: 0,
            glyph_position: Vector2::zero(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, Pod, Zeroable)]
pub struct GlyphType {
    data: u8,
}

pub struct Page {
    atlas: GlyphAtlas<GlyphType>,
    total_changes: usize,
}

impl Page {
    fn new(atlas: GlyphAtlas<GlyphType>) -> Self {
        Self {
            atlas,
            total_changes: 0,
        }
    }

    pub fn get_atlas(&self) -> &'_ GlyphAtlas<GlyphType> {
        &self.atlas
    }

    pub fn get_total_changes(&self) -> usize {
        self.total_changes
    }

    pub fn clear_total_changes(&mut self) {
        self.total_changes = 0;
    }
}

#[derive(Clone, Copy, Debug)]
struct LruListData {
    character: char,
    location: GlyphCacheLocation,
    time_id: usize,
}

pub struct GlyphCache {
    // font metrics
    font: fontdue::Font,
    font_size_em: f32,
    glyph_baseline: usize,
    glyph_size: Vector2<usize>,
    // glyph storage
    pages: Vec<Page>,
    unicode_page_grid_size: Vector2<usize>,
    max_pages: usize,
    free_glyph_location: GlyphCacheLocation,
    lru_node_lookup: HashMap<char,usize>,
    lru_node_list: LruList<LruListData>,
}

const ASCII_GLYPH_START: char = ' ';
const ASCII_GLYPH_END: char = '~';
const ASCII_TOTAL_GLYPHS: usize = ASCII_GLYPH_END as usize - ASCII_GLYPH_START as usize + 2;
// leave room for extra empty glyph cell at page=0,pos=(0,0)

impl GlyphCache {
    pub fn new(font: fontdue::Font, font_size_em: f32) -> Self {
        let font_line_metrics = font.horizontal_line_metrics(font_size_em).expect("Horizontal font expected");
        let glyph_baseline = font_line_metrics.ascent as usize;
        let glyph_height = (font_line_metrics.ascent - font_line_metrics.descent) as usize;
        let glyph_width = font.metrics(' ', font_size_em).advance_width as usize;
        let glyph_size = Vector2::<usize>::new(glyph_width, glyph_height);
 
        let mut pages = Vec::<Page>::new();
        // create single ascii page
        {
            let total_characters: usize = ASCII_TOTAL_GLYPHS;
            let grid_columns: usize = (total_characters as f32).sqrt().ceil() as usize;
            let grid_rows: usize = (total_characters as f32 / grid_columns as f32).ceil() as usize;
            let grid_size = Vector2::new(grid_columns, grid_rows);
            pages.push(Page::new(GlyphAtlas::new(glyph_size, grid_size)));
        }
        // create parameters for unicode pages
        // TODO: determine the best and maximum allowable page sizes for the cache???
        let unicode_page_grid_size = {
            let max_texture_width: usize = 512; // determine this properly
            let grid_columns = max_texture_width / glyph_size.x;
            let grid_rows = max_texture_width / glyph_size.y;
            Vector2::new(grid_columns, grid_rows)
        };
        let max_pages = 8;
 
        let mut cache = Self {
            // font metrics
            font,
            font_size_em,
            glyph_baseline,
            glyph_size,
            // glyph storage
            pages,
            unicode_page_grid_size,
            max_pages,
            free_glyph_location: GlyphCacheLocation::zero(),
            lru_node_lookup: HashMap::new(),
            lru_node_list: LruList::default(),
        };
        cache.generate_ascii_glyphs();
        cache
    }

    pub fn get_glyph_size(&self) -> Vector2<usize> {
        self.glyph_size
    }

    pub fn get_mut_pages(&mut self) -> &'_ mut [Page] {
        self.pages.as_mut_slice()
    }

    pub fn get_glyph_location(&mut self, c: char, time_id: usize) -> GlyphCacheLocation {
        // hardcode static ascii page
        if (ASCII_GLYPH_START..=ASCII_GLYPH_END).contains(&c) {
            return self.get_ascii_glyph_location(c);
        }
        // fallback to default glyph if not in atlas
        let c = if self.font.has_glyph(c) { c } else { '\0' };
        // check if glyph in cache
        if let Some(node_index) = self.lru_node_lookup.get(&c) {
            let node = self.lru_node_list.get_mut_data(*node_index);
            node.time_id = time_id;
            return node.location;
        }
        // generate glyph
        let location = self.free_glyph_location;
        // if we can still store or generate new pages then add a glyph to a free slot
        let mut is_cache_full = location.page_index >= self.pages.len();
        if is_cache_full {
            is_cache_full = !self.try_allocate_unicode_page();
        }
        if !is_cache_full {
            self.generate_glyph(c, location);
            let node_index = self.lru_node_list.push(&LruListData { 
                character: c, 
                location, 
                time_id,
            });
            self.lru_node_lookup.insert(c, node_index);
            self.free_glyph_location = self.get_next_free_location(location);
            return location;
        }
        // must replace lru glyph entry
        let node_index = self.lru_node_list.get_oldest().expect("Glyph cache should already be populated");
        let node = self.lru_node_list.get_mut_data(node_index);
        let old_character = node.character;
        let location = node.location;
        if node.time_id == time_id {
            // TODO: dynamically resize glyph cache to support more room if needed
            log::warn!("evicting glyph that is still in use '{}' with '{}' at time_id={}", old_character, c, time_id);
        }
        node.character = c;
        node.time_id = time_id;

        let _is_promoted = self.lru_node_list.promote(node_index);
        self.generate_glyph(c, location);
        self.lru_node_lookup.remove(&old_character);
        self.lru_node_lookup.insert(c, node_index);
        location
    }

    fn try_allocate_unicode_page(&mut self) -> bool {
        assert!(self.pages.len() <= self.max_pages);
        if self.pages.len() == self.max_pages {
            return false;
        }
        self.pages.push(Page::new(GlyphAtlas::new(self.glyph_size, self.unicode_page_grid_size)));
        log::info!("allocated a new glyph atlas page for total={}", self.pages.len());
        true
    }

    fn get_next_free_location(&self, mut location: GlyphCacheLocation) -> GlyphCacheLocation {
        let page = &self.pages[location.page_index];
        let grid_size = page.atlas.get_grid_size();
        location.glyph_position.x += 1;
        if location.glyph_position.x >= grid_size.x {
            location.glyph_position.x = 0;
            location.glyph_position.y += 1;
        }
        if location.glyph_position.y >= grid_size.y {
            location.glyph_position.x = 0;
            location.glyph_position.y = 0;
            location.page_index += 1;
        }
        location
    }

    fn generate_glyph(&mut self, c: char, location: GlyphCacheLocation) {
        let page = &mut self.pages[location.page_index];
        let glyph_view = page.atlas.get_mut_glyph_view(location.glyph_position);

        let (metrics, bitmap) = self.font.rasterize(c, self.font_size_em);
        // determine position of glyph from baseline
        let y_offset = self.glyph_baseline as i32 - metrics.ymin - metrics.height as i32;
        let x_offset = metrics.xmin;
        // If glyph overflows or underflows view we shifted it and clamp its location and size
        let y_offset = y_offset.max(0).min(glyph_view.size.y as i32) as usize;
        let x_offset = x_offset.max(0).min(glyph_view.size.x as i32) as usize;
        let height = metrics.height.min(glyph_view.size.y-y_offset);
        let width = metrics.width.min(glyph_view.size.x-x_offset);

        copy_bitmap_into_view(
            glyph_view,
            bitmap.as_slice(),
            Vector2::new(x_offset, y_offset),
            Vector2::new(width, height),
        );
        page.total_changes += 1;
    }

    // always generate ascii glyphs
    fn generate_ascii_glyphs(&mut self) {
        for c in ASCII_GLYPH_START..=ASCII_GLYPH_END {
            let location = self.get_ascii_glyph_location(c);
            assert!(location.page_index == 0);
            self.generate_glyph(c, location);
        }
        self.free_glyph_location = GlyphCacheLocation {
            page_index: 1,
            glyph_position: Vector2::new(0, 0),
        };
    }

    fn get_ascii_glyph_location(&self, c: char) -> GlyphCacheLocation {
        let page_index = 0;
        // leave room for empty glyph cell at pos=(0,0)
        let glyph_index = c as usize - ASCII_GLYPH_START as usize + 1;
        let grid_size = self.pages[page_index].atlas.get_grid_size();
        let glyph_column = glyph_index % grid_size.x;
        let glyph_row = glyph_index / grid_size.x;
        assert!(glyph_row < grid_size.y);
        let glyph_position = Vector2::new(glyph_column, glyph_row);
        GlyphCacheLocation { 
            page_index, 
            glyph_position,
        }
    }
}

fn copy_bitmap_into_view(view: MutableView2d<'_, GlyphType>, src: &[u8], offset: Vector2<usize>, size: Vector2<usize>) {
    // clear
    for y in 0..view.size.y {
        let i = y*view.row_stride;
        let row_dst = &mut view.data[i..(i+view.size.x)];
        row_dst.fill(GlyphType::zeroed());
    }

    assert!((offset.x + size.x) <= view.size.x);
    assert!((offset.y + size.y) <= view.size.y);
    for y in 0..size.y {
        let x_src = y*size.x;
        let row_src = &src[x_src..(x_src+size.x)];
        let y_dst = y + offset.y;
        let x_dst = y_dst*view.row_stride + offset.x;
        let row_dst = &mut view.data[x_dst..(x_dst+size.x)];
        for x in 0..size.x {
            row_dst[x].data = row_src[x];
        }
    }
}
 
