use std::borrow::Cow;
use bytemuck::{Pod, Zeroable};
use cgmath::{Vector2, Vector4};
use wgpu::util::DeviceExt;
use crate::glyph_atlas::GlyphAtlas;

#[repr(C)]
#[derive(Clone,Copy,Debug,Pod,Zeroable)]
pub struct CellData {
    pub atlas_index: Vector2<u16>,
    pub colour_foreground: Vector4<u8>,
    pub colour_background: Vector4<u8>,
    pub style_flags: u32,
}

impl Default for CellData {
    fn default() -> Self {
        Self::zeroed()
    }
}

#[repr(C)]
#[derive(Clone,Copy,Debug,Pod,Zeroable)]
struct GlobalParameters {
    render_scale: Vector2<f32>,
    grid_size: Vector2<u32>,
    atlas_size: Vector2<u32>,
}

impl Default for GlobalParameters {
    fn default() -> Self {
        Self {
            render_scale: Vector2::new(1.0,1.0),
            grid_size: Vector2::new(1,1),
            atlas_size: Vector2::new(1,1),
        }
    }
}

type Vertex = Vector2<f32>;

pub struct Renderer {
    shader_module: wgpu::ShaderModule,
    global_parameters: GlobalParameters,
    global_parameters_uniform: wgpu::Buffer,
    atlas_sampler: wgpu::Sampler,
    atlas_texture: wgpu::Texture,
    grid_texture: wgpu::Texture,
    mesh: Mesh,
    bind_group_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
    surface_texture_format: wgpu::TextureFormat,
    clear_colour: wgpu::Color,
}

impl Renderer {
    pub fn new(
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
    ) -> Self {
        // global shader parameters
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_text_shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });
        // global parameters
        let global_parameters = GlobalParameters::default();
        let global_parameters_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("global_parameters"),
            contents: bytemuck::cast_slice(&[global_parameters]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        // atlas
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("atlas_texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        // grid texture
        let grid_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("grid_texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        // mesh
        let mesh = Mesh::new(device);
        // bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        // shader pipeline
        let surface_texture_format = config.format;
        let clear_colour = wgpu::Color::BLACK; 
        // render pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[
                    mesh.get_vertex_buffer_layout(),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: surface_texture_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            shader_module,
            global_parameters,
            global_parameters_uniform,
            atlas_sampler,
            atlas_texture,
            grid_texture,
            mesh,
            bind_group_layout,
            render_pipeline,
            surface_texture_format,
            clear_colour,
        }
    }

    pub fn update_grid(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, cells: &[CellData], size: Vector2<usize>) {
        assert!(cells.len() == (size.x*size.y));
        let pixel_size_bytes = std::mem::size_of::<CellData>();
        assert!(pixel_size_bytes == 16);
        let extent = wgpu::Extent3d {
            width: size.x as u32,
            height: size.y as u32,
            depth_or_array_layers: 1,
        };
        let old_size = Vector2::new(
            self.grid_texture.width() as usize, 
            self.grid_texture.height() as usize,
        );
        if size != old_size {
            self.grid_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("grid_texture"),
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba32Uint,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
        }
        queue.write_texture(
            self.grid_texture.as_image_copy(),
            bytemuck::cast_slice(cells),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some((size.x*pixel_size_bytes) as u32),
                rows_per_image: Some(size.y as u32),
            },
            extent,
        );
        self.global_parameters.grid_size = size.cast::<u32>().unwrap();
        queue.write_buffer(&self.global_parameters_uniform, 0, bytemuck::cast_slice(&[self.global_parameters]));
    }

    pub fn update_atlas(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, atlas: &mut GlyphAtlas) {
        let pixel_size_bytes = 1;
        let texture_size = atlas.get_texture_size();
        let glyph_size = atlas.get_glyph_size();
        let total_glyphs_in_block = atlas.get_total_glyphs_in_block();
        let total_blocks = atlas.get_total_blocks();
        let atlas_size = Vector2::new(
            total_glyphs_in_block.x*total_blocks.x,
            total_glyphs_in_block.y*total_blocks.y,
        );
        let old_size = Vector2::new(
            self.atlas_texture.width() as usize, 
            self.atlas_texture.height() as usize,
        );
        if texture_size != old_size {
            self.atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("atlas_texture"),
                size: wgpu::Extent3d {
                    width: texture_size.x as u32,
                    height: texture_size.y as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
        }
        for block_index in atlas.get_modified_blocks() {
            let block_data = atlas.get_block(block_index);
            let block_size = Vector2::new(
                glyph_size.x*total_glyphs_in_block.x,
                glyph_size.y*total_glyphs_in_block.y,
            );
            let texture_region = wgpu::ImageCopyTexture {
                texture: &self.atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: (block_size.x*block_index.x) as u32,
                    y: (block_size.y*block_index.y) as u32,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            };
            queue.write_texture(
                texture_region,
                bytemuck::cast_slice(block_data),
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some((block_size.x*pixel_size_bytes) as u32),
                    rows_per_image: Some(block_size.y as u32),
                },
                wgpu::Extent3d {
                    width: block_size.x as u32,
                    height: block_size.y as u32,
                    depth_or_array_layers: 1,
                },
            );
        }
        atlas.clear_modified_count();

        self.global_parameters.atlas_size = atlas_size.cast::<u32>().unwrap();
        queue.write_buffer(&self.global_parameters_uniform, 0, bytemuck::cast_slice(&[self.global_parameters]));
    }

    pub fn update_render_scale(&mut self, queue: &wgpu::Queue, render_scale: Vector2<f32>) {
        self.global_parameters.render_scale = render_scale;
        queue.write_buffer(&self.global_parameters_uniform, 0, bytemuck::cast_slice(&[self.global_parameters]));
    }

    pub fn generate_commands(
        &mut self, 
        encoder: &mut wgpu::CommandEncoder, 
        render_output_view: &wgpu::TextureView,
        device: &wgpu::Device,
    ) {
        let atlas_texture_view = self.atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let grid_texture_view = self.grid_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind_group"),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.global_parameters_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.atlas_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&atlas_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&grid_texture_view),
                },
            ],
            layout: &self.bind_group_layout,
        });
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: render_output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.clear_colour),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));
        rpass.set_index_buffer(
            self.mesh.index_buffer.slice(..), 
            self.mesh.index_format,
        );
        rpass.set_bind_group(0, &bind_group, &[]);
        rpass.draw_indexed(0..self.mesh.total_indices as u32, 0, 0..1);
    }
}

struct Mesh {
    index_buffer: wgpu::Buffer,
    index_format: wgpu::IndexFormat,
    vertex_buffer: wgpu::Buffer,
    vertex_buffer_attributes: Vec<wgpu::VertexAttribute>,
    total_indices: usize,
}

impl Mesh {
    pub fn new(device: &wgpu::Device) -> Self {
        // vertex and index data for a quad
        let vertex_data: [Vertex; 4] = [
            Vertex::new(0.0, 0.0),
            Vertex::new(0.0, 1.0),
            Vertex::new(1.0, 1.0),
            Vertex::new(1.0, 0.0),
        ];
        let index_format = wgpu::IndexFormat::Uint16;
        let index_data: [u16; 6] = [
            0, 1, 2,
            2, 3, 0,
        ];
        let total_indices = index_data.len();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index_buffer"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX,
        });

        let vertex_buffer_attributes = wgpu::vertex_attr_array![
            0 => Float32x2,
        ].to_vec();

        Self {
            index_buffer,
            index_format,
            vertex_buffer,
            vertex_buffer_attributes,
            total_indices,
        }
    }

    pub fn get_vertex_buffer_layout(&self) -> wgpu::VertexBufferLayout<'_> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: self.vertex_buffer_attributes.as_slice(),
        }
    }
}

