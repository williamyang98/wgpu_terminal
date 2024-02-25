use std::borrow::Cow;

use crate::{
    render_params::RendererParameters,
    glyph_atlas::RendererGlyphAtlas,
    glyph_grid::RendererGlyphGrid,
    glyph_mesh::RendererGlyphMesh,
};


pub struct Renderer {
    // global shader parameters
    shader_module: wgpu::ShaderModule,
    render_params: RendererParameters,
    // components of shader
    glyph_atlas: RendererGlyphAtlas,
    glyph_mesh: RendererGlyphMesh,
    glyph_grid: RendererGlyphGrid,
    // shader pipeline
    render_pipeline: Option<wgpu::RenderPipeline>,
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
        let render_params = RendererParameters::new(device);
 
        // components
        let glyph_atlas = RendererGlyphAtlas::new(device);
        let glyph_mesh = RendererGlyphMesh::new(device);
        let glyph_grid = RendererGlyphGrid::new(device);

        // shader pipeline
        let surface_texture_format = config.format;
        let clear_colour = wgpu::Color::BLACK; 

        Self {
            // global shader parameters
            shader_module,
            render_params,
            // components
            glyph_atlas,
            glyph_mesh,
            glyph_grid,
            // shader pipeline
            render_pipeline: None,
            surface_texture_format,
            clear_colour,
        }
    }

    pub fn get_glyph_atlas(&mut self) -> &'_ mut RendererGlyphAtlas {
        &mut self.glyph_atlas
    }

    pub fn get_glyph_grid(&mut self) -> &'_ mut RendererGlyphGrid {
        &mut self.glyph_grid
    }

    pub fn get_render_params(&mut self) -> &'_ mut RendererParameters {
        &mut self.render_params
    }

    fn check_if_recreate_render_pipeline(&mut self) -> bool {
        self.glyph_atlas.get_and_clear_is_updated()
    }

    fn create_render_pipeline(&mut self, device: &wgpu::Device) {
        let glyph_atlas_bind_group_layout = self.glyph_atlas.get_bind_group_layout()
            .expect("Glyph atlas should be initialised");
        let glyph_grid_bind_group_layout = self.glyph_grid.get_bind_group_layout();
        let render_params_bind_group_layout = self.render_params.get_bind_group_layout();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_text_pipeline_layout"),
            bind_group_layouts: &[
                render_params_bind_group_layout,
                glyph_atlas_bind_group_layout,
                glyph_grid_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_text_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.shader_module,
                entry_point: "vs_main",
                buffers: &[
                    self.glyph_mesh.get_vertex_buffer_layout(),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.shader_module,
                entry_point: "fs_main",
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: self.surface_texture_format,
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
        self.render_pipeline = Some(render_pipeline);
    }

    pub fn generate_commands(
        &mut self, 
        encoder: &mut wgpu::CommandEncoder, 
        render_output_view: &wgpu::TextureView,
        device: &wgpu::Device,
    ) {
        if self.check_if_recreate_render_pipeline() {
            self.create_render_pipeline(device);
        }

        let render_pipeline = self.render_pipeline
            .as_ref()
            .expect("Pipeline must be created before render");
 
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
        rpass.set_pipeline(render_pipeline);
        rpass.set_vertex_buffer(0, self.glyph_mesh.get_vertex_buffer().slice(..));
        rpass.set_index_buffer(
            self.glyph_mesh.get_index_buffer().slice(..), 
            self.glyph_mesh.get_index_format(),
        );
        rpass.set_bind_group(0, self.render_params.get_bind_group(), &[]);
        rpass.set_bind_group(1, self.glyph_atlas.get_bind_group().unwrap(), &[]);
        rpass.set_bind_group(2, self.glyph_grid.get_bind_group().unwrap(), &[]);
        rpass.draw_indexed(0..self.glyph_mesh.get_total_indices() as u32, 0, 0..1);
    }
}
