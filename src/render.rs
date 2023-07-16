use wgpu::util::DeviceExt;
use {
    crate::{Globals, Particle},
    cgmath::{prelude::*, Matrix4, PerspectiveFov, Point3, Quaternion, Rad, Vector3},
    std::{collections::HashSet, f32::consts::PI, time::Instant},
    winit::{event, event_loop::ControlFlow},
};
pub mod state;
use state::State;

const TICKS_PER_FRAME: u32 = 3;
const PARTICLES_PER_GROUP: u32 = 256;
fn build_matrix(pos: Point3<f32>, dir: Vector3<f32>, aspect: f32) -> Matrix4<f32> {
    Matrix4::from(PerspectiveFov {
        fovy: Rad(PI / 2.0),
        aspect,
        near: 1E8,
        far: 1E14,
    }) * Matrix4::look_to_rh(pos, dir, Vector3::new(0.0, 1.0, 0.0))
}

pub async fn run(mut globals: Globals, particles: Vec<Particle>) {
    let mut state: State = State::new(globals, particles).await;
    let particles_size = (state.particles.len() * std::mem::size_of::<Particle>()) as u64;
    let work_group_count =
        ((state.particles.len() as f32) / (PARTICLES_PER_GROUP as f32)).ceil() as u32;

    let mut camera_dir: Vector3<f32> = Vector3::new(
        -state.display.camera_pos[0],
        -state.display.camera_pos[1],
        -state.display.camera_pos[2],
    );
    camera_dir = camera_dir.normalize();
    globals.matrix = build_matrix(
        state.display.camera_pos.into(),
        camera_dir,
        state.display.size.width as f32 / state.display.size.height as f32,
    )
    .into();
    let mut fly_speed = 1E10;
    let mut pressed_keys = HashSet::new();
    let mut right = camera_dir.cross(Vector3::new(0.0, 1.0, 0.0)).normalize();
    let mut last_tick = Instant::now();
    {
        let mut encoder =
            state
                .display
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Command Encoder"),
                });

        encoder.copy_buffer_to_buffer(
            &state.current_buffer_initializer,
            0,
            &state.current_buffer,
            0,
            particles_size,
        );

        state.display.queue.submit([encoder.finish()]);
    }

    state.event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
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
                event::WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }

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
                        event::VirtualKeyCode::Escape => {
                            *control_flow = ControlFlow::Exit;
                        }
                        _ => {}
                    }
                    pressed_keys.insert(keycode);
                }

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
                event::WindowEvent::Resized(new_size) => {
                    state.display.size = new_size;

                    state.display.resize(new_size.width, new_size.height);

                    let depth_texture =
                        state
                            .display
                            .device
                            .create_texture(&wgpu::TextureDescriptor {
                                label: Some("Depth Texture new"),
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

            event::Event::RedrawRequested(_window_id) => {
                let delta = last_tick.elapsed();
                let dt = delta.as_secs_f32();
                last_tick = Instant::now();

                let frame = state.display.surface.get_current_texture();
                let surface_texture = frame.ok().expect("Couldn't find frame texture!");
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

                camera_dir.normalize();
                right = camera_dir.cross(Vector3::new(0.0, 1.0, 0.0));
                right = right.normalize();

                let mut tmp: Point3<f32> = Point3::new(
                    state.display.camera_pos[0],
                    state.display.camera_pos[1],
                    state.display.camera_pos[2],
                );

                for c in pressed_keys.iter() {
                    match c {
                        event::VirtualKeyCode::W => {
                            tmp += camera_dir * fly_speed * dt;
                        }
                        event::VirtualKeyCode::A => {
                            tmp += -right * fly_speed * dt;
                        }
                        event::VirtualKeyCode::S => {
                            tmp += -camera_dir * fly_speed * dt;
                        }
                        event::VirtualKeyCode::D => {
                            tmp += right * fly_speed * dt;
                        }
                        event::VirtualKeyCode::Space => {
                            tmp[1] -= fly_speed * dt;
                        }
                        event::VirtualKeyCode::LShift => {
                            tmp[1] += fly_speed * dt;
                        }
                        _ => {}
                    }
                }
                globals.matrix = build_matrix(
                    tmp.into(),
                    camera_dir,
                    state.display.config.width as f32 / state.display.config.height as f32,
                )
                .into();
                state.display.camera_pos = [tmp[0], tmp[1], tmp[2]];

                let new_globals_buffer =
                    state
                        .display
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Globals Buffer"),
                            contents: bytemuck::cast_slice(&[globals]),
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_SRC,
                        });

                encoder.copy_buffer_to_buffer(
                    &new_globals_buffer,
                    0,
                    &state.globals_buffer,
                    0,
                    std::mem::size_of::<Globals>() as u64,
                );

                for _ in 0..TICKS_PER_FRAME {
                    encoder.copy_buffer_to_buffer(
                        &state.current_buffer,
                        0,
                        &state.old_buffer,
                        0,
                        particles_size,
                    );
                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Compute Pass"),
                    });
                    cpass.set_pipeline(&state.compute_pipeline);
                    cpass.set_bind_group(0, &state.bind_group, &[]);
                    cpass.dispatch_workgroups(work_group_count, 1, 1);
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
                    rpass.draw(0..state.particles.len() as u32, 0..1);
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
