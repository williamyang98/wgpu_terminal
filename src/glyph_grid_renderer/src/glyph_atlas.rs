use cgmath::{Vector2,Zero};
use glyph_grid::view_2d::ImmutableView2d;
use glyph_grid::glyph_cache::GlyphType;
use std::num::NonZeroU32;

pub type GridSize = Vector2<u32>;

pub struct RendererGlyphAtlas {
    // wgpu
    sampler: wgpu::Sampler,
    textures: Vec<wgpu::Texture>,
    grid_size_texture: wgpu::Texture,
    grid_size_data: Vec<GridSize>,
    bind_group_layout: Option<wgpu::BindGroupLayout>,
    bind_group: Option<wgpu::BindGroup>,
    // cpu
    total_pages: usize,
    is_updated: bool,
}

impl RendererGlyphAtlas {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let grid_size_texture = create_page_size_texture(device, 1);

        Self {
            sampler,
            textures: vec![],
            grid_size_texture, 
            grid_size_data: vec![], 
            bind_group_layout: None,
            bind_group: None,
            total_pages: 0,
            is_updated: false,
        }
    }

    pub(crate) fn get_and_clear_is_updated(&mut self) -> bool {
        let is_updated = self.is_updated;
        self.is_updated = false;
        is_updated
    }

    pub(crate) fn get_bind_group_layout(&self) -> Option<&wgpu::BindGroupLayout> {
        self.bind_group_layout.as_ref()
    }
 
    pub(crate) fn get_bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.bind_group.as_ref()
    }

    pub fn create_pages_if_changed(&mut self, device: &wgpu::Device, page_sizes: &[Vector2<usize>]) {
        let mut is_total_pages_changed = false;
        let new_total_pages = page_sizes.len();
        // push/pop until we get the right number of texture pages
        if new_total_pages < self.total_pages {
            // remove disused pages
            // we need to use .pop() since it doesnt require a Clone or Default trait bound
            let total_to_remove = self.total_pages-new_total_pages;
            for _ in 0..total_to_remove {
                let _ = self.textures.pop();
            }
        } else if new_total_pages > self.total_pages {
            // add new pages
            for i in self.total_pages..new_total_pages {
                let page_size = page_sizes[i];
                let texture = create_page_texture(device, page_size, i);
                self.textures.push(texture);
                log::info!("created new page texture index={} new_total_pages={}", i, new_total_pages);
            }
        }
        // resize grid_size texture
        if new_total_pages != self.total_pages {
            self.grid_size_data.resize(new_total_pages, GridSize::zero());
            self.grid_size_texture = create_page_size_texture(device, self.grid_size_data.len());
            log::info!("changed size of page grid_size_texture to width={}", new_total_pages);
            self.total_pages = new_total_pages;
            is_total_pages_changed = true;
        }

        assert!(self.textures.len() == self.total_pages);
        assert!(self.grid_size_data.len() == self.total_pages);
        assert!(self.grid_size_texture.width() as usize == self.total_pages);

        // replace pages if they are different sizes
        let mut is_page_textures_resized = false;
        for i in 0..self.total_pages {
            // update texture
            let old_texture = &self.textures[i];
            let old_page_size = Vector2::new(old_texture.width(), old_texture.height());
            let new_page_size = page_sizes[i];
            if old_page_size != new_page_size.cast::<u32>().unwrap() {
                self.textures[i] = create_page_texture(device, new_page_size, i);
                is_page_textures_resized = true;
            }
        }

        if is_total_pages_changed {
            self.bind_group_layout = Some(self.create_bind_group_layout(device, self.total_pages));
        }

        if is_total_pages_changed || is_page_textures_resized {
            self.is_updated = true;
            self.bind_group = Some(self.create_bind_group(device));
        }
    }

    pub fn update_page(
        &mut self, queue: &wgpu::Queue, 
        page_index: usize, page_view: ImmutableView2d<'_, GlyphType>, grid_size: Vector2<usize>
    ) {
        // write page texture
        assert!(page_index < self.total_pages);
        let texture = &self.textures[page_index];
        let extent = wgpu::Extent3d {
            width: page_view.size.x as u32,
            height: page_view.size.y as u32,
            depth_or_array_layers: 1,
        };
        assert!(extent.width == texture.width() && extent.height == texture.height());
        assert!(page_view.row_stride == page_view.size.x, "TODO: Handle when cpu source data has different row stride");
        let pixel_size_bytes = std::mem::size_of::<GlyphType>();
        queue.write_texture(
            texture.as_image_copy(),
            bytemuck::cast_slice(page_view.data),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(extent.width*pixel_size_bytes as u32),
                rows_per_image: Some(extent.height),
            },
            extent,
        );
        // write page grid_size 1d texture
        // grid_size is stored as vec2<u32> as RG32Uint
        let grid_size = grid_size.cast::<u32>().unwrap();
        self.grid_size_data[page_index] = grid_size;
        let texture = &self.grid_size_texture;
        let extent = wgpu::Extent3d {
            width: self.grid_size_data.len() as u32,
            height: 1,
            depth_or_array_layers: 1,
        };
        assert!(extent.width == texture.width());
        assert!(extent.height == texture.height());
        let pixel_size_bytes = std::mem::size_of::<Vector2<u32>>();
        queue.write_texture(
            texture.as_image_copy(),
            bytemuck::cast_slice(self.grid_size_data.as_slice()),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(extent.width*pixel_size_bytes as u32),
                rows_per_image: Some(extent.height),
            },
            extent,
        );
    }

    fn create_bind_group_layout(&self, device: &wgpu::Device, total_pages: usize) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glyph_atlas_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: NonZeroU32::new(total_pages as u32),
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D1,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        })
    }

    fn create_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        // synchronise to bind group layout and texture views and buffer array views
        let bind_group_layout = self.bind_group_layout.as_ref().expect("Bind group layout must be created");

        // extremely wierd way to get array of references
        let mut texture_views = Vec::<wgpu::TextureView>::new();
        for texture in &self.textures {
            texture_views.push(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        }
        let mut texture_views_ref = Vec::<&wgpu::TextureView>::new();
        for view in &texture_views {
            texture_views_ref.push(view);
        }
        let grid_size_texture_view = self.grid_size_texture.create_view(&wgpu::TextureViewDescriptor::default());

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glyph_atlas_bind_group"),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureViewArray(texture_views_ref.as_slice()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&grid_size_texture_view),
                },
            ],
            layout: bind_group_layout,
        })
    }
}

fn create_page_texture(device: &wgpu::Device, page_size: Vector2<usize>, page_index: usize) -> wgpu::Texture {
    let texture_format = wgpu::TextureFormat::R8Unorm;
    let texture_texel_size = texture_format.block_copy_size(None).unwrap() as usize;
    let cpu_texel_size = std::mem::size_of::<GlyphType>();
    if texture_texel_size != cpu_texel_size {
        panic!("Mismatching size between texture ({}) and cpu type ({})", texture_texel_size, cpu_texel_size);
    }
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(format!("glyph_atlas_page_texture_{}", page_index).as_str()),
        size: wgpu::Extent3d {
            width: page_size.x as u32,
            height: page_size.y as u32,
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

// Used to get Vector2<u32> which represents grid size for each page texture
fn create_page_size_texture(device: &wgpu::Device, total_pages: usize) -> wgpu::Texture {
    let texture_format = wgpu::TextureFormat::Rg32Uint;
    let texture_texel_size = texture_format.block_copy_size(None).unwrap() as usize;
    let cpu_texel_size = std::mem::size_of::<GridSize>();
    if texture_texel_size != cpu_texel_size {
        panic!("Mismatching size between texture ({}) and cpu type ({})", texture_texel_size, cpu_texel_size);
    }
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("glyph_atlas_page_size"),
        size: wgpu::Extent3d {
            width: total_pages as u32,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D1,
        format: texture_format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}
