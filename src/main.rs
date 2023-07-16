#![deny(
    rust_2018_compatibility,
    rust_2018_idioms,
    future_incompatible,
    nonstandard_style,
    unused,
    missing_copy_implementations,
    clippy::all
)]

mod config;
mod gen;
mod render;

use {
    cgmath::{Matrix4, Vector3},
    config::{Config, Construction},
    serde::{Deserialize, Serialize},
    std::f32::consts::PI,
};

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
#[repr(C)]
pub struct Particle {
    pos: [f32; 3],
    radius: f32,
    vel: [f32; 3],
    _p: f32,
    mass: f64,
    _p2: [f32; 2],
}

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Globals {
    matrix: [[f32; 4]; 4],
    camera_pos: [f32; 3],
    particles: u32,
    safety: f64,
    delta: f32,
    _p: f32,
}

impl Particle {
    fn new(pos: [f32; 3], vel: [f32; 3], mass: f64, density: f64) -> Self {
        Self {
            pos,
            radius: (3.0 * mass / (4.0 * density * PI as f64)).cbrt() as f32,
            vel,
            mass,
            _p: 0.0,
            _p2: [0.0; 2],
        }
    }
}

fn main() {
    let config = default_config();

    let particles = config.construct_particles();
    println!("Finished constructing particles.");

    let globals = Globals {
        matrix: Matrix4::from_translation(Vector3::new(0.0, 0.0, 0.0)).into(),
        camera_pos: config.camera_pos.into(),
        particles: particles.len() as u32,
        safety: config.safety,
        delta: 0.0,
        _p: 0.0,
    };
    pollster::block_on(render::run(globals, particles));
}

fn default_config() -> Config {
    Config {
        camera_pos: [0.0, 0.0, 1e10],
        safety: 1e20,
        constructions: vec![
            Construction::Galaxy {
                center_pos: [-1e11, -1e11, 0.0],
                center_vel: [10e6, 0.0, 0.0],
                center_mass: 1e35,
                amount: 100000,
                normal: [1.0, 0.0, 0.0],
            },
            Construction::Galaxy {
                center_pos: [1e11, 1e11, 0.0],
                center_vel: [0.0, 0.0, 0.0],
                center_mass: 3e35,
                amount: 100000,
                normal: [1.0, 1.0, 0.0],
            },
        ],
    }
}
