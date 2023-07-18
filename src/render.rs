use wgpu::{util::DeviceExt, SurfaceTexture};
use {
    crate::{GpuInfo, Particle},
    cgmath::{prelude::*, Matrix4, PerspectiveFov, Point3, Quaternion, Rad, Vector3},
    std::{collections::HashSet, f32::consts::PI, time::Instant},
    winit::{event, event_loop::ControlFlow},
};
pub mod state;
use state::State;

fn build_matrix(pos: Point3<f32>, dir: Vector3<f32>, aspect: f32) -> Matrix4<f32> {
    Matrix4::from(PerspectiveFov {
        fovy: Rad(PI / 2.0),
        aspect,
        near: 1E8,
        far: 1E14,
    }) * Matrix4::look_to_rh(pos, dir, Vector3::new(0.0, 1.0, 0.0))
}

pub async fn run(mut gpu_info: GpuInfo, particles: Vec<Particle>) {
    let mut state: State = State::new(gpu_info, particles).await;
    let n = state.particles.len();
    let p_size = (n * std::mem::size_of::<Particle>()) as u64;
    let workgroups = (n / 256) as u32;

    let mut cam: Vector3<f32> = Vector3::new(
        -state.display.camera_pos[0],
        -state.display.camera_pos[1],
        -state.display.camera_pos[2],
    );
    cam = cam.normalize();
    gpu_info.matrix = build_matrix(
        state.display.camera_pos.into(),
        cam,
        state.display.size.width as f32 / state.display.size.height as f32,
    )
    .into();
    let mut vel = 1E10;
    let mut keys = HashSet::new();
    let mut right = cam.cross(Vector3::new(0.0, 1.0, 0.0)).normalize();
    let mut update = Instant::now();
    {
        let mut encoder =
            state
                .display
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Command Encoder"),
                });

        encoder.copy_buffer_to_buffer(&state.cur_init, 0, &state.cur, 0, p_size);

        state.display.queue.submit([encoder.finish()]);
    }

    state.event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            event::Event::DeviceEvent {
                event: event::DeviceEvent::MouseMotion { delta },
                ..
            } => {
                cam = Quaternion::from_angle_y(Rad(-delta.0 as f32 / 300.0)).rotate_vector(cam);
                cam = Quaternion::from_axis_angle(right, Rad(delta.1 as f32 / 300.0))
                    .rotate_vector(cam);
            }

            event::Event::WindowEvent { event, .. } => match event {
                event::WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }

                event::WindowEvent::KeyboardInput {
                    input:
                        event::KeyboardInput {
                            virtual_keycode: Some(key),
                            state: event::ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    match key {
                        event::VirtualKeyCode::Escape => {
                            *control_flow = ControlFlow::Exit;
                        }
                        _ => {}
                    }
                    keys.insert(key);
                }

                event::WindowEvent::KeyboardInput {
                    input:
                        event::KeyboardInput {
                            virtual_keycode: Some(key),
                            state: event::ElementState::Released,
                            ..
                        },
                    ..
                } => {
                    keys.remove(&key);
                }
                event::WindowEvent::MouseWheel { delta, .. } => {
                    vel *= (1.0
                        + (match delta {
                            event::MouseScrollDelta::LineDelta(_, i) => i as f32 / 8.0,
                            event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 64.0,
                        }))
                    .min(4.0)
                    .max(0.25);

                    vel = vel.min(1E13).max(1E9);
                }
                event::WindowEvent::Resized(resized) => {
                    state.display.size = resized;

                    state.display.resize(resized.width, resized.height);

                    let depth_texture =
                        state
                            .display
                            .device
                            .create_texture(&wgpu::TextureDescriptor {
                                label: Some("New Depth Texture"),
                                size: wgpu::Extent3d {
                                    width: state.display.config.width,
                                    height: state.display.config.height,
                                    depth_or_array_layers: 1,
                                },
                                view_formats: &[],
                                mip_level_count: 1,
                                sample_count: 1,
                                dimension: wgpu::TextureDimension::D2,
                                format: wgpu::TextureFormat::Depth32Float,
                                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                            });
                    state.depth_view =
                        depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
                }
                _ => {}
            },

            event::Event::RedrawRequested(_) => {
                let dt = update.elapsed().as_secs_f32();
                update = Instant::now();
                let surface_texture: SurfaceTexture = state
                    .display
                    .surface
                    .get_current_texture()
                    .ok()
                    .expect("no frame texture");
                let view = surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder =
                    state
                        .display
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Command Encoder"),
                        });

                cam.normalize();
                right = cam.cross(Vector3::new(0.0, 1.0, 0.0));
                right = right.normalize();

                let mut tmp: Point3<f32> = Point3::new(
                    state.display.camera_pos[0],
                    state.display.camera_pos[1],
                    state.display.camera_pos[2],
                );

                for i in keys.iter() {
                    match i {
                        event::VirtualKeyCode::W => {
                            tmp += cam * vel * dt;
                        }
                        event::VirtualKeyCode::A => {
                            tmp += -right * vel * dt;
                        }
                        event::VirtualKeyCode::S => {
                            tmp += -cam * vel * dt;
                        }
                        event::VirtualKeyCode::D => {
                            tmp += right * vel * dt;
                        }
                        event::VirtualKeyCode::Space => {
                            tmp[1] -= vel * dt;
                        }
                        event::VirtualKeyCode::LShift => {
                            tmp[1] += vel * dt;
                        }
                        _ => {}
                    }
                }
                gpu_info.matrix = build_matrix(
                    tmp.into(),
                    cam,
                    state.display.config.width as f32 / state.display.config.height as f32,
                )
                .into();
                state.display.camera_pos = [tmp[0], tmp[1], tmp[2]];

                let new_gpu_info =
                    state
                        .display
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("GpuInfo Buffer"),
                            contents: bytemuck::cast_slice(&[gpu_info]),
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_SRC,
                        });

                encoder.copy_buffer_to_buffer(
                    &new_gpu_info,
                    0,
                    &state.gpu_buffer,
                    0,
                    std::mem::size_of::<GpuInfo>() as u64,
                );

                for _ in 0..3 {
                    encoder.copy_buffer_to_buffer(&state.cur, 0, &state.prev, 0, p_size);
                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Compute Pass"),
                    });
                    cpass.set_pipeline(&state.comp_pipeline);
                    cpass.set_bind_group(0, &state.bind_group, &[]);
                    cpass.dispatch_workgroups(workgroups, 1, 1);
                }
                {
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
                            view: &state.depth_view,
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

                    rpass.set_pipeline(&state.render_pipeline);
                    rpass.set_bind_group(0, &state.bind_group, &[]);
                    rpass.draw(0..n as u32, 0..1);
                }
                drop(view);
                state.display.queue.submit([encoder.finish()]);
                surface_texture.present();
                state
                    .display
                    .surface
                    .configure(&state.display.device, &state.display.config);
            }
            event::Event::MainEventsCleared => {
                state.display.window.request_redraw();
            }
            _ => {}
        }
    });
}
