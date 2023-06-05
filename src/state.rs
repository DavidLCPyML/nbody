use std::iter;

use wgpu::util::DeviceExt;
use cgmath::prelude::*;

use winit::window::Window;
use winit::event::*;

mod camera;
mod instance;
mod vertex;

pub struct State {
  surface: wgpu::Surface,
  device: wgpu::Device,
  queue: wgpu::Queue,
  config: wgpu::SurfaceConfiguration,
  pub size: winit::dpi::PhysicalSize<u32>,
  render_pipeline: wgpu::RenderPipeline,
  vertex_buffer: wgpu::Buffer,
  index_buffer: wgpu::Buffer,
  num_indices: u32,
  window: Window,
  camera: camera::Camera,
  camera_uniform: camera::CameraUniform,
  camera_buffer: wgpu::Buffer,
  camera_bind_group: wgpu::BindGroup,
  camera_controller: camera::CameraController,
  instances: Vec<instance::Instance>,
  instance_buffer: wgpu::Buffer,
  projection: camera::Projection,
  pub mouse_pressed: bool,
}

impl State {
  pub fn get_camera_controller(&mut self) -> &mut camera::CameraController {
    &mut self.camera_controller
  }
  pub async fn new(window: Window) -> Self {
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
                  features: wgpu::Features::empty(),
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
          .find(|f| f.describe().srgb)
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

      let instances = (0..instance::NUM_INSTANCES_PER_ROW).flat_map(|z| {
          (0..instance::NUM_INSTANCES_PER_ROW).map(move |x| {
              let position = cgmath::Vector3 {x: x as f32, y: 0.0, z: z as f32} - instance::INSTANCE_DISPLACEMENT;
              let rotation = if position.is_zero() {
                  cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
              } else {
                  cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
              };
              instance::Instance {
                  position, rotation,
              }
          })
      }).collect::<Vec<_>>();

      let instance_data = instances.iter().map(instance::Instance::to_raw).collect::<Vec<_>>();
      let instance_buffer = device.create_buffer_init(
          &wgpu::util::BufferInitDescriptor {
              label: Some("Instance Buffer"),
              contents: bytemuck::cast_slice(&instance_data),
              usage: wgpu::BufferUsages::VERTEX,
          }
      );

      let camera = camera::Camera::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
      let projection = camera::Projection::new(config.width, config.height, cgmath::Deg(45.0), 0.1, 100.0);
      let mut camera_uniform = camera::CameraUniform::new();
      camera_uniform.update_view_proj(&camera, &projection);

      let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
          label: Some("Camera Buffer"),
          contents: bytemuck::cast_slice(&[camera_uniform]),
          usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      });

      let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
              }
          ],
          label: Some("camera_bind_group_layout"),
      });

      let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
          layout: &camera_bind_group_layout,
          entries: &[
              wgpu::BindGroupEntry {
                  binding: 0,
                  resource: camera_buffer.as_entire_binding(),
              }
          ],
          label: Some("camera_bind_group"),
      });
      let camera_controller = camera::CameraController::new(1.0, 0.4);
      let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
          label: Some("Shader"),
          source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
      });

      let render_pipeline_layout =
          device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
              label: Some("Render Pipeline Layout"),
              bind_group_layouts: &[&camera_bind_group_layout],
              push_constant_ranges: &[],
          });

      let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
          label: Some("Render Pipeline"),
          layout: Some(&render_pipeline_layout),
          vertex: wgpu::VertexState {
              module: &shader,
              entry_point: "vs_main",
              buffers: &[vertex::Vertex::desc(), instance::InstanceRaw::desc()],
          },
          fragment: Some(wgpu::FragmentState {
              module: &shader,
              entry_point: "fs_main",
              targets: &[Some(wgpu::ColorTargetState {
                  format: config.format,
                  blend: Some(wgpu::BlendState {
                      color: wgpu::BlendComponent::REPLACE,
                      alpha: wgpu::BlendComponent::REPLACE,
                  }),
                  write_mask: wgpu::ColorWrites::ALL,
              })],
          }),
          primitive: wgpu::PrimitiveState {
              topology: wgpu::PrimitiveTopology::TriangleList,
              strip_index_format: None,
              front_face: wgpu::FrontFace::Ccw,
              cull_mode: None,
              polygon_mode: wgpu::PolygonMode::Fill,
              unclipped_depth: false,
              conservative: false,
          },
          depth_stencil: None,
          multisample: wgpu::MultisampleState {
              count: 1,
              mask: !0,
              alpha_to_coverage_enabled: false,
          },
          multiview: None,
      });

      let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
          label: Some("Vertex Buffer"),
          contents: bytemuck::cast_slice(vertex::VERTICES),
          usage: wgpu::BufferUsages::VERTEX,
      });
      let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
          label: Some("Index Buffer"),
          contents: bytemuck::cast_slice(vertex::INDICES),
          usage: wgpu::BufferUsages::INDEX,
      });
      let num_indices = vertex::INDICES.len() as u32;

      Self {
          surface,
          device,
          queue,
          config,
          size,
          render_pipeline,
          vertex_buffer,
          index_buffer,
          num_indices,
          window,
          camera,
          camera_uniform,
          camera_buffer,
          camera_bind_group,
          camera_controller,
          instances,
          instance_buffer,
          projection,
          mouse_pressed: false,
      }
  }

  pub fn window(&self) -> &Window {
      &self.window
  }

  pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
      if new_size.width > 0 && new_size.height > 0 {
          self.size = new_size;
          self.config.width = new_size.width;
          self.config.height = new_size.height;
          self.surface.configure(&self.device, &self.config);
          self.projection.resize(new_size.width, new_size.height);
      }
  }

  pub fn input(&mut self, event: &WindowEvent) -> bool {
      match event {
          WindowEvent::KeyboardInput {
              input:
                  KeyboardInput {
                      virtual_keycode: Some(key),
                      state,
                      ..
                  },
              ..
          } => self.camera_controller.process_keyboard(*key, *state),
          WindowEvent::MouseWheel { delta, .. } => {
              self.camera_controller.process_scroll(delta);
              true
          }
          WindowEvent::MouseInput {
              button: MouseButton::Left,
              state,
              ..
          } => {
              self.mouse_pressed = *state == ElementState::Pressed;
              true
          }
          _ => false,
      }
  }

  pub fn update(&mut self, dt: instant::Duration) {
      self.camera_controller.update_camera(&mut self.camera, dt);
      self.camera_uniform.update_view_proj(&self.camera, &self.projection);
      self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
  }

  pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
      let output = self.surface.get_current_texture()?;
      let view = output
          .texture
          .create_view(&wgpu::TextureViewDescriptor::default());

      let mut encoder = self
          .device
          .create_command_encoder(&wgpu::CommandEncoderDescriptor {
              label: Some("Render Encoder"),
          });

      {
          let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
              label: Some("Render Pass"),
              color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                  view: &view,
                  resolve_target: None,
                  ops: wgpu::Operations {
                      load: wgpu::LoadOp::Clear(wgpu::Color {
                          r: 0.1,
                          g: 0.2,
                          b: 0.3,
                          a: 1.0,
                      }),
                      store: true,
                  },
              })],
              depth_stencil_attachment: None,
          });

          render_pass.set_pipeline(&self.render_pipeline);
          render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
          render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
          render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
          render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
          render_pass.draw_indexed(0..self.num_indices, 0, 0..self.instances.len() as _);        }

      self.queue.submit(iter::once(encoder.finish()));
      output.present();

      Ok(())
  }
}
