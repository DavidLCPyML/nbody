use {
    wgpu::util::DeviceExt,
    crate::{Globals, Particle},
    winit::{event_loop::EventLoop, window::WindowBuilder},
};

pub struct State {
    pub globals: Globals,
    pub particles: Vec<Particle>,
    pub old_buffer: wgpu::Buffer,
    pub current_buffer: wgpu::Buffer,
    pub current_buffer_initializer: wgpu::Buffer,
    pub globals_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub compute_pipeline: wgpu::ComputePipeline,
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
    pub async fn new(globals: Globals, particles: Vec<Particle>) -> Self {
        let particles_size = (particles.len() * std::mem::size_of::<Particle>()) as u64;

        let event_loop = EventLoop::new();

        let window = WindowBuilder::new()
            .with_title(env!("CARGO_PKG_NAME"))
            .build(&event_loop)
            .ok()
            .unwrap();
        let display = Display::new(window).await.unwrap();

        display
            .window()
            .set_cursor_grab(winit::window::CursorGrabMode::Confined)
            .ok();
        display.window().set_cursor_visible(false);

        let cs = include_bytes!("../shader.comp.spv");
        let cs_module = unsafe {
            display
                .device
                .create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
                    label: Some("Compute Shader"),
                    source: wgpu::util::make_spirv_raw(cs),
                })
        };

        let vs = include_bytes!("../shader.vert.spv");
        let vs_module = unsafe {
            display
                .device
                .create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
                    label: Some("Vertex Shader"),
                    source: wgpu::util::make_spirv_raw(vs),
                })
        };

        let fs = include_bytes!("../shader.frag.spv");
        let fs_module = unsafe {
            display
                .device
                .create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
                    label: Some("Fragment Shader"),
                    source: wgpu::util::make_spirv_raw(fs),
                })
        };

        let globals_buffer = display
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Globals Buffer"),
                contents: bytemuck::cast_slice(&[globals]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let mut initial_particle_data = vec![0.0f32; (particles.len() * 12) as usize];
        let mut i = 0;
        for particle_instance_chunk in initial_particle_data.chunks_mut(12) {
            particle_instance_chunk[0] = particles[i].pos[0];
            particle_instance_chunk[1] = particles[i].pos[1];
            particle_instance_chunk[2] = particles[i].pos[2];
            particle_instance_chunk[3] = particles[i]._p1;
            particle_instance_chunk[4] = particles[i].vel[0];
            particle_instance_chunk[5] = particles[i].vel[1];
            particle_instance_chunk[6] = particles[i].vel[2];
            particle_instance_chunk[7] = particles[i]._p2;
            let mass_arr: [f32; 2] = bytemuck::cast_slice(&[particles[i].mass])
                .try_into()
                .unwrap();
            let calib_arr: [f32; 2] = bytemuck::cast_slice(&[particles[i].calibrate])
                .try_into()
                .unwrap();
            particle_instance_chunk[8] = mass_arr[0];
            particle_instance_chunk[9] = mass_arr[1];
            particle_instance_chunk[10] = calib_arr[0];
            particle_instance_chunk[11] = calib_arr[1];
            i += 1;
        }
        let old_buffer = display.device.create_buffer(&wgpu::BufferDescriptor {
            size: particles_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::MAP_READ,
            label: Some("Old Buffer"),
            mapped_at_creation: false,
        });
        let current_buffer_initializer =
            display
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Current Buffer Initializer"),
                    contents: bytemuck::cast_slice(&initial_particle_data),
                    usage: wgpu::BufferUsages::COPY_SRC,
                });
        let current_buffer = display.device.create_buffer(&wgpu::BufferDescriptor {
            size: particles_size,
            usage: wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
            label: Some("Current Buffer"),
            mapped_at_creation: false,
        });
        let depth_texture = display.device.create_texture(&wgpu::TextureDescriptor {
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
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group_layout =
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
                                    std::mem::size_of::<Globals>() as _,
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
        let bind_group = display
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: globals_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: old_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: current_buffer.as_entire_binding(),
                    },
                ],
            });
        let pipeline_layout =
            display
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline =
            display
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Compute Pipeline"),
                    module: &cs_module,
                    entry_point: "main",
                    layout: Some(&pipeline_layout),
                });
        let render_pipeline =
            display
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Render Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &vs_module,
                        entry_point: "main",
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &fs_module,
                        entry_point: "main",
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
            globals,
            particles,
            old_buffer,
            current_buffer,
            current_buffer_initializer,
            globals_buffer,
            bind_group,
            compute_pipeline,
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
