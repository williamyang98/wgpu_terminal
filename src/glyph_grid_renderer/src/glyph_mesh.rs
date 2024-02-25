use bytemuck::{Pod, Zeroable};
use cgmath::Vector2;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    position: Vector2<f32>,
}

impl Vertex {
    fn new(x: f32, y: f32) -> Self {
        Self {
            position: Vector2::new(x, y),
        }
    }
}

pub struct RendererGlyphMesh {
    vertex_buffer_attributes: Vec<wgpu::VertexAttribute>,
    index_buffer: wgpu::Buffer,
    index_format: wgpu::IndexFormat,
    vertex_buffer: wgpu::Buffer,
    total_indices: usize,
}

impl RendererGlyphMesh {
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
            label: Some("glyph_mesh_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("glyph_mesh_index_buffer"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX,
        });

        let vertex_buffer_attributes = wgpu::vertex_attr_array![
            0 => Float32x2,
        ].to_vec();

        Self {
            vertex_buffer_attributes,
            index_buffer,
            index_format,
            vertex_buffer,
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

    pub fn get_vertex_buffer(&self) -> &'_ wgpu::Buffer {
        &self.vertex_buffer
    }


    pub fn get_index_buffer(&self) -> &'_ wgpu::Buffer {
        &self.index_buffer
    }

    pub fn get_index_format(&self) -> wgpu::IndexFormat {
        self.index_format
    }

    pub fn get_total_indices(&self) -> usize {
        self.total_indices 
    }
}

