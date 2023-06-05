#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

pub const BLUE:[f32; 3] = [0.722, 0.22, 0.231];

pub const _RED: [f32; 3] = [0.44, 0.0,  0.22];

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2, // NEW!
                },
            ],
        }
    }
}

pub const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.25, 0.0, 0.0],
        color: BLUE,
    }, // A
    Vertex {
        position: [0.0, 0.25, 0.0],
        color: BLUE,
    }, // B
    Vertex {
        position: [0.0, -0.25, 0.0],
        color: BLUE,
    }, // C
];
pub const INDICES: &[u16] = &[0, 1, 2, /* padding */ 0];
