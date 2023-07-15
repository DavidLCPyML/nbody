use wgpu::{util::DeviceExt, Error};

use {
    crate::{Globals, Particle},
    cgmath::{prelude::*, Matrix4, PerspectiveFov, Point3, Quaternion, Rad, Vector3},
    std::{collections::HashSet, f32::consts::PI, time::Instant},
    winit::{
        event,
        event_loop::{ControlFlow, EventLoop},
        window::{Window, WindowBuilder},
    },
};

const TICKS_PER_FRAME: u32 = 3; // steps
const PARTICLES_PER_GROUP: u32 = 256;
fn build_matrix(pos: Point3<f32>, dir: Vector3<f32>, aspect: f32) -> Matrix4<f32> {
    Matrix4::from(PerspectiveFov {
        fovy: Rad(PI / 2.0),
        aspect,
        near: 1E8,
        far: 1E14,
    }) * Matrix4::look_to_rh(pos, dir, Vector3::new(0.0, 1.0, 0.0))
}

pub struct Display {
    surface: wgpu::Surface,
    pub window: Window,
    pub config: wgpu::SurfaceConfiguration,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::Adapter,
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl Display {
    pub async fn new(window: Window) -> Result<Self, Error> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });
        let surface = unsafe { instance.create_surface(&window) }.unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::SPIRV_SHADER_PASSTHROUGH
                        | wgpu::Features::VERTEX_WRITABLE_STORAGE
                        | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                },
                None,
            )
            .await
            .unwrap();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        Ok(Self {
            surface,
            window,
            config,
            device,
            queue,
            adapter,
            size,
        })
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }
}

pub async fn run(mut globals: Globals, particles: Vec<Particle>) {
    // How many bytes do the particles need
    let particles_size = (particles.len() * std::mem::size_of::<Particle>()) as u64;

    let work_group_count = ((particles.len() as f32) / (PARTICLES_PER_GROUP as f32)).ceil() as u32;

    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title(env!("CARGO_PKG_NAME"))
        .build(&event_loop)
        .ok()
        .unwrap();
    let mut display = Display::new(window).await.unwrap();

    // Try to grab mouse
    let _ = display
        .window()
        .set_cursor_grab(winit::window::CursorGrabMode::Confined);
    display.window().set_cursor_visible(false);

    let cs = include_bytes!("shader.comp.spv");
    let cs_module = unsafe {
        display
            .device
            .create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
                label: Some("Compute Shader"),
                source: wgpu::util::make_spirv_raw(cs),
            })
    };

    let vs = include_bytes!("shader.vert.spv");
    let vs_module = unsafe {
        display
            .device
            .create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
                label: Some("Vertex Shader"),
                source: wgpu::util::make_spirv_raw(vs),
            })
    };

    let fs = include_bytes!("shader.frag.spv");
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
        particle_instance_chunk[3] = particles[i].radius;
        particle_instance_chunk[4] = particles[i].vel[0];
        particle_instance_chunk[5] = particles[i].vel[1];
        particle_instance_chunk[6] = particles[i].vel[2];
        particle_instance_chunk[7] = particles[i]._p;
        let mass_arr: [f32; 2] = bytemuck::cast_slice(&[particles[i].mass])
            .try_into()
            .unwrap();
        particle_instance_chunk[8] = mass_arr[0];
        particle_instance_chunk[9] = mass_arr[1];
        particle_instance_chunk[10] = particles[i]._p2[0];
        particle_instance_chunk[11] = particles[i]._p2[1];
        i += 1;
    }

    println!(
        "random particle: {:?}",
        initial_particle_data[0..12].to_vec()
    );
    println!("random particle: {:?}", particles[0]);
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
    println!(
        "Current_buffer_initializer size: {:?}",
        current_buffer_initializer.size()
    );

    let current_buffer = display.device.create_buffer(&wgpu::BufferDescriptor {
        size: particles_size,
        usage: wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::STORAGE,
        label: Some("Current Buffer"),
        mapped_at_creation: false,
    });
    println!("Current_buffer size: {:?}", current_buffer.size());

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
    let mut depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Describe the buffers that will be available to the GPU
    let bind_group_layout =
        display
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Bind Group Layout"),
                entries: &[
                    // Globals
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                std::mem::size_of::<Globals>() as _
                            ),
                        },
                        count: None,
                    },
                    // Old Particle data
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
                    // Current Particle data
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
                // Globals
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals_buffer.as_entire_binding(),
                },
                // Old Particle data
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: old_buffer.as_entire_binding(),
                },
                // Current Particle data
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: current_buffer.as_entire_binding(),
                },
            ],
        });
    // println!("Bind_group created: {:?}", bind_group);

    // Combine all bind_group_layouts
    let pipeline_layout = display
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

    // Create compute pipeline
    let compute_pipeline =
        display
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline"),
                module: &cs_module,
                entry_point: "main",
                layout: Some(&pipeline_layout),
            });
    // Create render pipeline
    let render_pipeline = display
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

    let mut camera_dir: Vector3<f32> = Vector3::new(
        -globals.camera_pos[0],
        -globals.camera_pos[1],
        -globals.camera_pos[2],
    );
    camera_dir = camera_dir.normalize();
    globals.matrix = build_matrix(
        globals.camera_pos.into(),
        camera_dir,
        display.size.width as f32 / display.size.height as f32,
    )
    .into();
    let mut fly_speed = 1E10;
    let mut pressed_keys = HashSet::new();
    let mut right = camera_dir.cross(Vector3::new(0.0, 1.0, 0.0)).normalize();
    let mut last_tick = Instant::now();
    {
        let mut encoder = display
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command Encoder"),
            });

        // Initialize current particle buffer
        encoder.copy_buffer_to_buffer(
            &current_buffer_initializer,
            0,
            &current_buffer,
            0,
            particles_size,
        );

        display.queue.submit([encoder.finish()]);
    }

    // Start main loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = if cfg!(feature = "metal-auto-capture") {
            ControlFlow::Exit
        } else {
            ControlFlow::Poll
        };
        match event {
            // Move mouse
            event::Event::DeviceEvent {
                event: event::DeviceEvent::MouseMotion { delta },
                ..
            } => {
                camera_dir = Quaternion::from_angle_y(Rad(-delta.0 as f32 / 300.0))
                    .rotate_vector(camera_dir);
                camera_dir = Quaternion::from_axis_angle(right, Rad(delta.1 as f32 / 300.0))
                    .rotate_vector(camera_dir);
            }

            event::Event::WindowEvent { event, .. } => match event {
                // Close window
                event::WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }

                // Keyboard input
                event::WindowEvent::KeyboardInput {
                    input:
                        event::KeyboardInput {
                            virtual_keycode: Some(keycode),
                            state: event::ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    // timo key stuff
                    match keycode {
                        // Exit
                        event::VirtualKeyCode::Escape => {
                            *control_flow = ControlFlow::Exit;
                        }
                        event::VirtualKeyCode::Key0 => {
                            globals.delta = 0.0;
                        }
                        event::VirtualKeyCode::Key1 => {
                            globals.delta = 1E0;
                        }
                        event::VirtualKeyCode::Key2 => {
                            globals.delta = 2E0;
                        }
                        event::VirtualKeyCode::Key3 => {
                            globals.delta = 4E0;
                        }
                        event::VirtualKeyCode::Key4 => {
                            globals.delta = 8E0;
                        }
                        event::VirtualKeyCode::Key5 => {
                            globals.delta = 16E0;
                        }
                        event::VirtualKeyCode::Key6 => {
                            globals.delta = 32E0;
                        }
                        event::VirtualKeyCode::F => {
                            let delta = last_tick.elapsed();
                            println!("delta: {:?}, fps: {:.2}", delta, 1.0 / delta.as_secs_f32());
                        }
                        event::VirtualKeyCode::F11 => {
                            if display.window().fullscreen().is_some() {
                                display.window().set_fullscreen(None);
                            } else {
                                display.window().set_fullscreen(Some(
                                    winit::window::Fullscreen::Borderless(
                                        display.window().primary_monitor(),
                                    ),
                                ));
                            }
                        }
                        _ => {}
                    }
                    pressed_keys.insert(keycode);
                }

                // Release key
                event::WindowEvent::KeyboardInput {
                    input:
                        event::KeyboardInput {
                            virtual_keycode: Some(keycode),
                            state: event::ElementState::Released,
                            ..
                        },
                    ..
                } => {
                    pressed_keys.remove(&keycode);
                }

                // Mouse scroll
                event::WindowEvent::MouseWheel { delta, .. } => {
                    fly_speed *= (1.0
                        + (match delta {
                            event::MouseScrollDelta::LineDelta(_, c) => c as f32 / 8.0,
                            event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 64.0,
                        }))
                    .min(4.0)
                    .max(0.25);

                    fly_speed = fly_speed.min(1E13).max(1E9);
                }

                // Resize window
                event::WindowEvent::Resized(new_size) => {
                    display.size = new_size;

                    display.resize(new_size.width, new_size.height);

                    let depth_texture = display.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("Depth Texture new"),
                        size: wgpu::Extent3d {
                            width: display.config.width,
                            height: display.config.height,
                            depth_or_array_layers: 1,
                        },
                        view_formats: &[],
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Depth32Float,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    });
                    depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
                }
                _ => {}
            },

            event::Event::RedrawRequested(_window_id) => {
                let delta = last_tick.elapsed();
                let dt = delta.as_secs_f32();
                last_tick = Instant::now();

                let frame = display.surface.get_current_texture();
                let surface_texture = frame.ok().expect("Couldn't find frame texture!");
                let view = surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder =
                    display
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Command Encoder"),
                        });

                camera_dir.normalize();
                right = camera_dir.cross(Vector3::new(0.0, 1.0, 0.0));
                right = right.normalize();

                let mut tmp: Point3<f32> = Point3::new(
                    globals.camera_pos[0],
                    globals.camera_pos[1],
                    globals.camera_pos[2],
                );

                if pressed_keys.contains(&event::VirtualKeyCode::A) {
                    tmp += -right * fly_speed * dt;
                }

                if pressed_keys.contains(&event::VirtualKeyCode::D) {
                    tmp += right * fly_speed * dt;
                    // println!("D key pressed")
                }

                if pressed_keys.contains(&event::VirtualKeyCode::W) {
                    tmp += camera_dir * fly_speed * dt;
                }

                if pressed_keys.contains(&event::VirtualKeyCode::S) {
                    tmp += -camera_dir * fly_speed * dt;
                }

                if pressed_keys.contains(&event::VirtualKeyCode::Space) {
                    tmp[1] -= fly_speed * dt;
                }

                if pressed_keys.contains(&event::VirtualKeyCode::LShift) {
                    tmp[1] += fly_speed * dt;
                }

                globals.matrix = build_matrix(
                    tmp.into(),
                    camera_dir,
                    display.config.width as f32 / display.config.height as f32,
                )
                .into();
                globals.camera_pos = [tmp[0], tmp[1], tmp[2]];

                // Create new globals buffer
                let new_globals_buffer =
                    display
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Globals Buffer"),
                            contents: bytemuck::cast_slice(&[globals]),
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_SRC,
                        });

                encoder.copy_buffer_to_buffer(
                    &new_globals_buffer,
                    0,
                    &globals_buffer,
                    0,
                    std::mem::size_of::<Globals>() as u64,
                );

                // Compute the simulation a few times
                for _ in 0..TICKS_PER_FRAME {
                    encoder.copy_buffer_to_buffer(
                        &current_buffer,
                        0,
                        &old_buffer,
                        0,
                        particles_size,
                    );
                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Compute Pass"),
                    });
                    cpass.set_pipeline(&compute_pipeline);
                    cpass.set_bind_group(0, &bind_group, &[]);
                    cpass.dispatch_workgroups(work_group_count, 1, 1);
                }

                {
                    // Render the current state
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.03,
                                    g: 0.03,
                                    b: 0.03,
                                    a: 1.0,
                                }),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &depth_view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: true,
                            }),
                            stencil_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(0),
                                store: true,
                            }),
                        }),
                    });

                    rpass.set_pipeline(&render_pipeline);
                    rpass.set_bind_group(0, &bind_group, &[]);
                    rpass.draw(0..particles.len() as u32, 0..1);
                }
                drop(view);

                display.queue.submit([encoder.finish()]);
                surface_texture.present();
                display.surface.configure(&display.device, &display.config);
            }

            // No more events in queue
            event::Event::MainEventsCleared => {
                display.window.request_redraw();
            }
            _ => {}
        }
    });
}
