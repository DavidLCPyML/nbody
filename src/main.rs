#![deny(nonstandard_style, unused)]

mod gen;
mod render;

use {
    cgmath::{Matrix4, Vector3},
    serde::{Deserialize, Serialize},
};

const CALIBRATE: f64 = 1E20;

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
#[repr(C)]
pub struct Particle {
    pos: [f32; 3],
    _pad1: f32,
    vel: [f32; 3],
    _pad2: f32,
    mass: f64,
    calibrate: f64,
}

#[derive(Deserialize, Clone, Debug, Copy)]
pub enum Galaxy {
    Particle {
        pos: [f32; 3],
        vel: [f32; 3],
        mass: f64,
    },
    Init {
        center_pos: [f32; 3],
        center_vel: [f32; 3],
        center_mass: f64,
        amount: u32,
        normal: [f32; 3],
    },
}

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct GpuInfo {
    matrix: [[f32; 4]; 4],
    particles: u32,
    motion: f32,
    _pad1: [f32; 2],
}

impl Particle {
    fn new(pos: [f32; 3], vel: [f32; 3], mass: f64, calibrate: f64) -> Self {
        Self {
            pos,
            vel,
            mass,
            calibrate,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }
}

pub fn init_galaxy(calibrate: f64, galaxies: Vec<Galaxy>) -> Vec<Particle> {
    let mut particles = Vec::new();
    for c in &galaxies {
        particles.push(match c {
            Galaxy::Particle { pos, vel, mass } => {
                Particle::new((*pos).into(), (*vel).into(), *mass, calibrate)
            }
            Galaxy::Init {
                center_pos,
                center_vel,
                center_mass,
                ..
            } => Particle::new(
                (*center_pos).into(),
                (*center_vel).into(),
                *center_mass,
                calibrate,
            ),
        })
    }

    for i in &galaxies {
        if let Galaxy::Init {
            center_pos,
            center_vel,
            center_mass,
            amount,
            normal,
        } = i
        {
            gen::formation(
                &mut particles,
                *amount,
                CALIBRATE,
                (*center_pos).into(),
                (*center_vel).into(),
                *center_mass,
                (*normal).into(),
            );
        }
    }
    particles
}

fn main() {
    let galaxies: Vec<Galaxy> = vec![
        Galaxy::Init {
            center_pos: [-5e10, -5e10, 0.0],
            center_vel: [10e6, 0.0, 0.0],
            center_mass: 1e35,
            amount: 100000,
            normal: [1.0, 0.0, 0.0],
        },
        Galaxy::Init {
            center_pos: [5e10, 5e10, 0.0],
            center_vel: [0.0, 0.0, 0.0],
            center_mass: 3e35,
            amount: 100000,
            normal: [1.0, 1.0, 0.0],
        },
    ];

    let particles = init_galaxy(CALIBRATE, galaxies);
    let gpu_info = GpuInfo {
        matrix: Matrix4::from_translation(Vector3::new(0.0, 0.0, 0.0)).into(),
        particles: particles.len() as u32,
        motion: 6.0,
        _pad1: [0.0; 2],
    };
    pollster::block_on(render::run(gpu_info, particles));
}
