use {
    crate::{GpuInfo, Particle},
    wgpu::util::DeviceExt,
    winit::{event_loop::EventLoop, window::WindowBuilder},
};

pub struct State {
    pub gpu_info: GpuInfo,
    pub particles: Vec<Particle>,
    pub prev: wgpu::Buffer,
    pub cur: wgpu::Buffer,
    pub cur_init: wgpu::Buffer,
    pub gpu_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub comp_pipeline: wgpu::ComputePipeline,
    pub render_pipeline: wgpu::RenderPipeline,
    pub depth_texture: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub pipeline_layout: wgpu::PipelineLayout,
    pub event_loop: EventLoop<()>,
    pub display: Display,
}

pub mod display;
use display::Display;

impl State {
    pub async fn new(gpu_info: GpuInfo, particles: Vec<Particle>) -> Self {
        let p_size: u64 = (particles.len() * std::mem::size_of::<Particle>()) as u64;
        let event_loop: EventLoop<()> = EventLoop::new();
        let window: winit::window::Window = WindowBuilder::new()
            .with_title(env!("CARGO_PKG_NAME"))
            .build(&event_loop)
            .ok()
            .unwrap();
        let display: Display = Display::new(window).await.unwrap();
        let cs_mod: wgpu::ShaderModule = display
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../compute.wgsl").into()),
            });
        let vs_mod: wgpu::ShaderModule = display
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Vertex Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../vertex.wgsl").into()),
            });
        let fs_mod: wgpu::ShaderModule = display
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Fragment Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../fragment.wgsl").into()),
            });

        let gpu_buffer: wgpu::Buffer = display
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("GpuInfo Buffer"),
                contents: bytemuck::cast_slice(&[gpu_info]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let mut init_particle: Vec<f32> = vec![0.0f32; (particles.len() * 12) as usize];
        let mut i: usize = 0;
        for chunk in init_particle.chunks_mut(12) {
            chunk[0] = particles[i].pos[0];
            chunk[1] = particles[i].pos[1];
            chunk[2] = particles[i].pos[2];
            chunk[3] = particles[i]._pad1;
            chunk[4] = particles[i].vel[0];
            chunk[5] = particles[i].vel[1];
            chunk[6] = particles[i].vel[2];
            chunk[7] = particles[i]._pad2;
            chunk[8] = particles[i].mass;
            chunk[9] = particles[i].calibrate;
            chunk[10] = particles[i]._pad3[0];
            chunk[11] = particles[i]._pad3[1];
            i += 1;
        }
        let prev: wgpu::Buffer = display.device.create_buffer(&wgpu::BufferDescriptor {
            size: p_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::MAP_READ,
            label: Some("Old Buffer"),
            mapped_at_creation: false,
        });
        let cur_init: wgpu::Buffer = display
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Current Buffer Initializer"),
                contents: bytemuck::cast_slice(&init_particle),
                usage: wgpu::BufferUsages::COPY_SRC,
            });
        let cur: wgpu::Buffer = display.device.create_buffer(&wgpu::BufferDescriptor {
            size: p_size,
            usage: wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
            label: Some("Current Buffer"),
            mapped_at_creation: false,
        });
        let depth_texture: wgpu::Texture = display.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: display.config.width,
                height: display.config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            view_formats: &[],
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        });
        let depth_view: wgpu::TextureView = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group_layout: wgpu::BindGroupLayout =
            display
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: wgpu::BufferSize::new(
                                    std::mem::size_of::<GpuInfo>() as _,
                                ),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: wgpu::BufferSize::new(
                                    std::mem::size_of::<Particle>() as _,
                                ),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: wgpu::BufferSize::new(
                                    std::mem::size_of::<Particle>() as _,
                                ),
                            },
                            count: None,
                        },
                    ],
                });
        let bind_group: wgpu::BindGroup = display
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: gpu_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: prev.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: cur.as_entire_binding(),
                    },
                ],
            });
        let pipeline_layout: wgpu::PipelineLayout =
            display
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let comp_pipeline: wgpu::ComputePipeline =
            display
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Compute Pipeline"),
                    module: &cs_mod,
                    entry_point: "main",
                    layout: Some(&pipeline_layout),
                });
        let render_pipeline: wgpu::RenderPipeline =
            display
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Render Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &vs_mod,
                        entry_point: "vs_main",
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &fs_mod,
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: display.config.format,
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent::REPLACE,
                                alpha: wgpu::BlendComponent::REPLACE,
                            }),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::PointList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: None,
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        bias: wgpu::DepthBiasState {
                            constant: 0,
                            slope_scale: 0.0,
                            clamp: 0.0,
                        },
                        depth_compare: wgpu::CompareFunction::LessEqual,
                        stencil: wgpu::StencilState {
                            front: wgpu::StencilFaceState::IGNORE,
                            back: wgpu::StencilFaceState::IGNORE,
                            read_mask: 0,
                            write_mask: 0,
                        },
                    }),
                    multisample: wgpu::MultisampleState {
                        count: 1,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                });

        Self {
            gpu_info,
            particles,
            prev,
            cur,
            cur_init,
            gpu_buffer,
            bind_group,
            comp_pipeline,
            render_pipeline,
            depth_texture,
            depth_view,
            bind_group_layout,
            pipeline_layout,
            event_loop,
            display,
        }
    }
}
