use bytemuck::{Pod, Zeroable};
use cgmath::Vector2;
use glyph_grid::view_2d::ImmutableView2d;
use glyph_grid::glyph_grid::GlyphGridData;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Parameters {
    grid_size: Vector2<u32>,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            grid_size: Vector2::new(0, 0),
        }
    }
}

pub struct RendererGlyphGrid {
    params_uniform_buffer: wgpu::Buffer,
    texture: Option<wgpu::Texture>,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: Option<wgpu::BindGroup>,
    params: Parameters,
}

impl RendererGlyphGrid {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let params = Parameters::default();
        let params_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("glyph_grid_uniform"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group_layout = Self::create_bind_group_layout(device);

        Self {
            params_uniform_buffer,
            texture: None,
            bind_group_layout,
            bind_group: None,
            params,
        }
    }

    pub(crate) fn get_bind_group_layout(&self) -> &'_ wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub(crate) fn get_bind_group(&self) -> Option<&'_ wgpu::BindGroup> {
        self.bind_group.as_ref()
    }

    pub fn update_grid(&mut self, queue: &wgpu::Queue, device: &wgpu::Device, view: ImmutableView2d<'_,GlyphGridData>) {
        let grid_size = view.size.cast::<u32>().unwrap();
        assert!(grid_size.x >= 1);
        assert!(grid_size.y >= 1);
        if self.texture.is_none() || self.params.grid_size != grid_size {
            self.texture = Some(Self::create_texture(device, view.size));
            self.bind_group = Some(self.create_bind_group(device));
            self.params.grid_size = grid_size;
            self.submit_params_uniform_update(queue);
        }

        let texture = self.texture.as_ref().expect("Texture must be created before submitting data");
        let extent = wgpu::Extent3d {
            width: grid_size.x,
            height: grid_size.y,
            depth_or_array_layers: 1,
        };
        assert!(extent.width == texture.width() && extent.height == texture.height());
        assert!(view.row_stride == view.size.x, "TODO: Handle when cpu source data has different row stride");
        let pixel_size_bytes = std::mem::size_of::<GlyphGridData>();
        queue.write_texture(
            texture.as_image_copy(),
            bytemuck::cast_slice(view.data),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(extent.width*pixel_size_bytes as u32),
                rows_per_image: Some(extent.height),
            },
            extent,
        );
    }

    fn submit_params_uniform_update(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.params_uniform_buffer, 
            0, 
            bytemuck::cast_slice(&[self.params]),
        );
    }

    fn create_texture(device: &wgpu::Device, size: Vector2<usize>) -> wgpu::Texture {
        let texture_format = wgpu::TextureFormat::Rgba32Uint;
        let texture_texel_size = texture_format.block_copy_size(None).unwrap() as usize;
        let cpu_texel_size = std::mem::size_of::<GlyphGridData>();
        if texture_texel_size != cpu_texel_size {
            panic!("Mismatching size between texture ({}) and cpu type ({})", texture_texel_size, cpu_texel_size);
        }
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph_grid_texture"),
            size: wgpu::Extent3d {
                width: size.x as u32,
                height: size.y as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glyph_grid_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
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
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        })
    }

    fn create_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        let texture = self.texture.as_ref().expect("Texture must be created before creating bind group");
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glyph_grid_bind_group"),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.params_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
            ],
            layout: &self.bind_group_layout,
        })
    }
}

