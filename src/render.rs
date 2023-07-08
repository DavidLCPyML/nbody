//! This module handles everything that has to do with the window. That includes opening a window,
//! parsing events and rendering. See shader.comp for the physics simulation algorithm.

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

const TICKS_PER_FRAME: u32 = 3; // Number of simulation steps per redraw
const PARTICLES_PER_GROUP: u32 = 256; // REMEMBER TO CHANGE SHADER.COMP
const NUM_PARTICLES: u32 = 1500;

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
                    features: wgpu::Features::SPIRV_SHADER_PASSTHROUGH | wgpu::Features::VERTEX_WRITABLE_STORAGE | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
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
        // Shader code in this tutorial assumes an Srgb surface texture. Using a different
        // one will result all the colors comming out darker. If you want to support non
        // Srgb surfaces, you'll need to account for that when drawing to the frame.
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
        .build(&event_loop).ok().unwrap();
    let mut display = Display::new(window).await.unwrap();
    
    // Try to grab mouse
    // let _ = display.window().set_cursor_grab(winit::window::CursorGrabMode::Confined);
    // display.window().set_cursor_visible(false);
    

    // Load compute shader for the simulation
    let cs = include_bytes!("shader.comp.spv");
    let cs_module = unsafe {
        display.device.create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV{
            label: Some("Compute Shader"),
            source: wgpu::util::make_spirv_raw(cs),
            })
    };

    // Load vertex shader to set calculate perspective, size and position of particles
    let vs = include_bytes!("shader.vert.spv");
    let vs_module = unsafe {
        display.device.create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV{
            label: Some("Vertex Shader"),
            source: wgpu::util::make_spirv_raw(vs),
            })
    };

    // Load fragment shader
    let fs = include_bytes!("shader.frag.spv");
    let fs_module = unsafe {
        display.device.create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV{
            label: Some("Fragment Shader"),
            source: wgpu::util::make_spirv_raw(fs),
            })
    };

    // Create globals buffer to give global information to the shader
    let globals_buffer = display.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Globals Buffer"),
        contents: bytemuck::cast_slice(&[globals]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    println!("Globals_buffer: {:?}", globals_buffer);

    

    // Texture to keep track of which particle is in front (for the camera)
    // let depth_texture = display.device.create_texture(&wgpu::TextureDescriptor {
    //     label: Some("Depth Texture"),
    //     size: wgpu::Extent3d {
    //         width: display.config.width,
    //         height: display.config.height,
    //         depth_or_array_layers: 1,
    //     },
    //     mip_level_count: 1,
    //     sample_count: 1,
    //     dimension: wgpu::TextureDimension::D2,
    //     format: wgpu::TextureFormat::Depth32Float,
    //     view_formats: &[],
    //     usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    // });
    // let mut depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Describe the buffers that will be available to the GPU
    let bind_group_layout = display.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                        std::mem::size_of::<Globals>() as _,)
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
    println!("Bind_group_layout: {:?}", bind_group_layout);

    let compute_bind_group_layout =
        display.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (sim_param_data.len() * std::mem::size_of::<f32>()) as _,
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
                        min_binding_size: wgpu::BufferSize::new((NUM_PARTICLES * 16) as _),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new((NUM_PARTICLES * 16) as _),
                    },
                    count: None,
                },
            ],
            label: None,
        });
    let compute_pipeline_layout =
        display.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("compute"),
            bind_group_layouts: &[&compute_bind_group_layout],
            push_constant_ranges: &[],
        });

    let render_pipeline_layout =
        display.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let mut particle_buffers = Vec::<wgpu::Buffer>::new();
        let mut particle_bind_groups = Vec::<wgpu::BindGroup>::new();
            for i in 0..2 {
                particle_buffers.push(
                    display.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("Particle Buffer {i}")),
                        contents: bytemuck::cast_slice(&initial_particle_data),
                        usage: wgpu::BufferUsages::VERTEX
                            | wgpu::BufferUsages::STORAGE
                            | wgpu::BufferUsages::COPY_DST,
                    }),
                );
            }
    
            // create two bind groups, one for each buffer as the src
            // where the alternate buffer is used as the dst
    
            for i in 0..2 {
                particle_bind_groups.push(display.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &compute_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: sim_param_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: particle_buffers[i].as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: particle_buffers[(i + 1) % 2].as_entire_binding(), // bind to opposite buffer
                        },
                    ],
                    label: None,
                }));
            }

    let render_pipeline = display.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vs_module,
            entry_point: "main_vs",
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: 4 * 4,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                },
                wgpu::VertexBufferLayout {
                    array_stride: 2 * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![2 => Float32x2],
                },
            ],
        },
        fragment: Some(wgpu::FragmentState {
            module: &fs_module,
            entry_point: "main_fs",
            targets: &[Some(display.config.view_formats[0].into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    // Create compute pipeline
    let compute_pipeline = display.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute pipeline"),
        layout: Some(&compute_pipeline_layout),
        module: &cs_module,
        entry_point: "main",
    });

    // Create render pipeline
    let render_pipeline = display.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vs_module,
            entry_point: "main_vs",
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: 4 * 4,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                },
                wgpu::VertexBufferLayout {
                    array_stride: 2 * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![2 => Float32x2],
                },
            ],
        },
        fragment: Some(wgpu::FragmentState {
            module: &fs_module,
            entry_point: "main_fs",
            targets: &[Some(display.config.view_formats[0].into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    // Where is the camera looking at?

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
    ).into();

    println!("Initial camera position: {:?}", globals.camera_pos);
    println!("Initial camera direction: {:?}", camera_dir);
    println!("Initial camera matrix: {:?}", globals.matrix);

    // Speed of the camera
    let mut fly_speed = 1E10;

    // Which keys are currently held down?
    let mut pressed_keys = HashSet::new();

    // Vector that points to the right of the camera
    let mut right = camera_dir.cross(Vector3::new(0.0, 1.0, 0.0)).normalize();

    // Time of the last tick
    let mut last_tick = Instant::now();

    // Initial setup
    {
        let mut encoder =
            display.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Command Encoder") });

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
                                display.window().set_fullscreen(Some(winit::window::Fullscreen::Borderless(
                                    display.window().primary_monitor(),
                                )));
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

                    // Reset swap chain, it's outdated
                    display.resize(new_size.width, new_size.height);

                    // Reset depth texture
                    // let depth_texture = display.device.create_texture(&wgpu::TextureDescriptor {
                    //     label: Some("Depth Texture new"),
                    //     size: wgpu::Extent3d {
                    //         width: display.config.width,
                    //         height: display.config.height,
                    //         depth_or_array_layers: 1,
                    //     },
                    //     view_formats: &[],
                    //     mip_level_count: 1,
                    //     sample_count: 1,
                    //     dimension: wgpu::TextureDimension::D2,
                    //     format: wgpu::TextureFormat::Depth32Float,
                    //     usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    // });
                    // depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
                }
                _ => {}
            },

            // Simulate and redraw
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
                    display.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Command Encoder"),});

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
                ).into();
                globals.camera_pos = [tmp[0], tmp[1], tmp[2]];

                // Create new globals buffer
                let new_globals_buffer = display.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Globals Buffer"),
                    contents: bytemuck::cast_slice(&[globals]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_SRC,
                });

                // Upload the new globals buffer to the GPU
                encoder.copy_buffer_to_buffer(
                    &new_globals_buffer,
                    0,
                    &globals_buffer,
                    0,
                    std::mem::size_of::<Globals>() as u64,
                );
                
                // println!("camera_dir: {:?}, right: {:?}, tmp: {:?}", camera_dir, right, tmp);
                // println!("globals updated to: {:?}, {:?}", globals.camera_pos, globals.matrix);

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
                        label: Some("Compute pass"),
                    });                    
                    cpass.set_pipeline(&compute_pipeline);
                    cpass.set_bind_group(0, &, &[]);
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
                        depth_stencil_attachment: None,
                    });

                    rpass.set_pipeline(&render_pipeline);
                    rpass.set_bind_group(0, &particle_bind_groups[self.frame_num % 2], &[]);
                    rpass.draw(0..particles.len() as u32, 0..1);
                }
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
