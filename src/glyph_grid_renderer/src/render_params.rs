use bytemuck::{Pod, Zeroable};
use cgmath::Vector2;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Parameters {
    render_scale: Vector2<f32>,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            render_scale: Vector2::new(1.0, 1.0),
        }
    }
}

pub struct RendererParameters {
    data: Parameters,
    uniform_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl RendererParameters {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let data = Parameters::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("render_params_uniform"),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render_params_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render_params_bind_group"),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
            layout: &bind_group_layout,
        });

        Self {
            data,
            uniform_buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub(crate) fn get_bind_group_layout(&self) -> &'_ wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub(crate) fn get_bind_group(&self) -> &'_ wgpu::BindGroup {
        &self.bind_group
    }

    pub fn set_render_scale(&mut self, queue: &wgpu::Queue, render_scale: Vector2<f32>) {
        if render_scale != self.data.render_scale {
            self.data.render_scale = render_scale;
            self.submit_update(queue);
        }
    }

    fn submit_update(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.data]));
    }
}
