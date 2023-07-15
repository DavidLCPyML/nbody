struct Globals {
  matrix: mat4x4<f32>,
  camera_pos: vec4<f32>,
  particles: u32,
  safety: f64,
  delta: f32,
  _p: f32,
};

@group(0) @binding(0) var<uniform> globals: Globals;

@vertex
fn main_vs(
    @location(0) particle_pos: vec2<f32>,
    @location(1) particle_vel: vec2<f32>,
    @location(2) position: vec2<f32>,
) -> @builtin(position) vec4<f32> {
    let angle = -atan2(particle_vel.x, particle_vel.y);
    let pos = vec2<f32>(
        position.x * cos(angle) - position.y * sin(angle),
        position.x * sin(angle) + position.y * cos(angle)
    );
    return vec4<f32>(pos + particle_pos, 0.0, 1.0);
}

@fragment
fn main_fs() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
