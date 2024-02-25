struct RenderParameters {
    render_scale: vec2<f32>,
}

struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct GlyphGridParams {
    grid_size: vec2<u32>,
}

struct FragmentInput {
    @builtin(position) position: vec4<f32>,
    @location(0) grid_position: vec2<f32>,
}

// Refer to glyph_grid::GlyphGridData for how this is packed
struct Cell {
    glyph_page_index: u32,
    glyph_position: vec2<u32>,
    colour_foreground: vec4<u32>,
    colour_background: vec4<u32>,
    is_underline: bool,
}

fn unpack_cell_data(data: vec4<u32>) -> Cell {
    var d: Cell;
    d.glyph_page_index =    (data.r & 0x000000FF);
    d.glyph_position.x =    (data.r & 0x000FFF00) >> 8;
    d.glyph_position.y =    (data.r & 0xFFF00000) >> 20;
    d.colour_foreground.r = (data.g & 0x000000FF);
    d.colour_foreground.g = (data.g & 0x0000FF00) >> 8;
    d.colour_foreground.b = (data.g & 0x00FF0000) >> 16;
    d.colour_foreground.a = (data.g & 0xFF000000) >> 32;
    d.colour_background.r = (data.b & 0x000000FF);
    d.colour_background.g = (data.b & 0x0000FF00) >> 8;
    d.colour_background.b = (data.b & 0x00FF0000) >> 16;
    d.colour_background.a = (data.b & 0xFF000000) >> 32;
    d.is_underline =    bool(data.a & 0x00000001);
    return d;
}

@group(0) @binding(0) var<uniform> render_params: RenderParameters;
@group(1) @binding(0) var glyph_atlas_sampler: sampler;
@group(1) @binding(1) var glyph_atlas_textures: binding_array<texture_2d<f32>>;
@group(1) @binding(2) var glyph_atlas_grid_size: texture_1d<u32>;
@group(2) @binding(0) var<uniform> glyph_grid_params: GlyphGridParams;
@group(2) @binding(1) var glyph_grid_texture: texture_2d<u32>;

@vertex
fn vs_main(vertex: VertexInput) -> FragmentInput {
    let screen_vertex_position = vertex.position*render_params.render_scale;
    let frag_position = vec4<f32>(screen_vertex_position.x*2.0 - 1.0, -(screen_vertex_position.y*2.0 - 1.0), 0.0, 1.0);
    let grid_position = vertex.position;

    var frag_out: FragmentInput;
    frag_out.position = frag_position;
    frag_out.grid_position = grid_position;
    return frag_out;
}

@fragment
fn fs_main(frag: FragmentInput) -> @location(0) vec4<f32> {
    // get location of grid
    let absolute_grid_position = frag.grid_position*vec2<f32>(glyph_grid_params.grid_size);
    let absolute_grid_position_floor = floor(absolute_grid_position);
    let absolute_grid_offset = absolute_grid_position - absolute_grid_position_floor;

    // get grid cell data
    let cell_data = textureLoad(glyph_grid_texture, vec2<i32>(absolute_grid_position_floor), 0);
    let cell = unpack_cell_data(cell_data);

    // determine glyph atlas location
    let glyph_atlas_grid_size = textureLoad(glyph_atlas_grid_size, i32(cell.glyph_page_index), 0).xy;
    let glyph_size = 1.0 / vec2<f32>(glyph_atlas_grid_size);
    let glyph_offset = glyph_size * vec2<f32>(cell.glyph_position);
    let glyph_position = absolute_grid_offset*glyph_size + glyph_offset;

    // fetch glyph data from atlas 
    let data = textureSampleLevel(glyph_atlas_textures[cell.glyph_page_index], glyph_atlas_sampler, glyph_position, 0.0);
    let v: f32 = data.r;
    let foreground_colour = vec4<f32>(cell.colour_foreground) / 255.0;
    let background_colour = vec4<f32>(cell.colour_background) / 255.0;
    let output_colour = foreground_colour*v + background_colour*(1-v);
    return output_colour;
}

