use cgmath::{
    prelude::*,
    {Vector3},
};
use std::f32::consts::PI;
use rand::prelude::*;

const G: f64 = 6.67408E-11;
const ARMS: u32 = 4;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Particle {
  pos: [f64; 3],
  velocity: [f64; 3],
  pub mass: f64,
  pub density: f64,
}

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Globals {
    camera_x: f32, // 1
    camera_y: f32, // 2
    camera_z: f32, // 3
    particles: u32, // 4
    safety: f64, // 5, 6
    delta: f32, // 7
    _p: f32, // 8
}

impl Default for Globals {
  fn default() -> Self {
    Globals {
      camera_x: 0.0,
      camera_y: 0.0,
      camera_z: 1.0,
      particles: 0,
      safety: 1.0,
      delta: 0.0001,
      _p: 0.0001,
    }
  }
}

impl Particle {
    pub fn new(pos: [f64; 3], velocity: [f64; 3], mass: f64, density: f64) -> Self {
        Self {
            pos,
            velocity,
            mass,
            density,
        }
    }
}

pub enum Galaxy {
  Particle {
    pos: [f64; 3],
    velocity: [f64; 3],
    mass: f64,
    density: f64,
  },
  Structures {
    num_particles: u32,
    center_pos: [f64; 3],
    center_mass: f64,
    center_velocity: [f64; 3],
    center_density: f64,
    normal: [f64; 3],
  }
}

pub struct Galaxies {
  pub galaxy: Vec<Galaxy>,
}

impl Galaxies {
  pub fn new(&self) -> Vec<Particle> {
    let mut particles = Vec::new();
    for i in &self.galaxy {
      particles.push(match i {
        Galaxy::Particle {
          pos,
          velocity,
          mass,
          density,
        } => Particle::new(*pos, *velocity, *mass, *density),
        Galaxy::Structures {
          num_particles,
          center_pos,
          center_mass,
          center_velocity,
          center_density,
          normal,
        } => Particle::new(*center_pos, *center_velocity, *center_mass, *center_density),
      })
    }
    for i in &self.galaxy {
      if let Galaxy::Structures {
        num_particles,
        center_pos,
        center_mass,
        center_velocity,
        center_density,
        normal,
      } = i
      {
        create_galaxy(
          &mut particles,
          *num_particles,
          (*center_pos).into(),
          *center_mass,
          (*center_velocity).into(),
          *center_density,
          *normal,
        );
      }
    }
    particles
  }
}

pub fn create_galaxy(
    particles: &mut Vec<Particle>,
    num_particles: u32,
    center_pos: Vector3<f64>,
    center_mass: f64,
    center_velocity: Vector3<f64>,
    _center_density: f64,
    normal: [f64; 3],
) {
  let mut normal: Vector3<f64> = normal.try_into().unwrap();

  normal = normal.normalize();
  let tangent: Vector3<f64> = normal.cross(Vector3::new(normal.y, -normal.x, normal.z));
  let bitangent = normal.cross(tangent);
  let radius: f64 = 3E12 as f64;
  let mass = 1e30;
  let density = 1e20;

  for _ in 0..num_particles / 5 {
    let theta: f64 = thread_rng().gen::<f64>() * 2.0 * PI as f64;
    let dir: Vector3<f64> = tangent * theta.cos() + bitangent * theta.cos();
    let pos = center_pos + dir * radius * theta.cos();
    let speed = (G * center_mass / radius).sqrt();
    let fly_dir = dir.cross(normal); // check if necessary
    let velocity = center_velocity + fly_dir * speed;
    let pos: [f64; 3] = pos.try_into().unwrap();
    let velocity: [f64; 3] = velocity.try_into().unwrap();
    particles.push(Particle::new(pos, velocity, mass, density));
  }

  for _ in 0..num_particles / 5 * 4 {
    let arm: u32 = thread_rng().gen_range(0..ARMS);

    let theta: f64 = (arm / ARMS) as f64 * 2.0 * PI as f64;
    let dir: Vector3<f64> = tangent * theta.cos() + bitangent * theta.sin();
    let pos = center_pos + dir * radius * theta.cos();
    let speed = (G * center_mass / radius).sqrt();
    let fly_dir = dir.cross(normal); // check if necessary
    let velocity = center_velocity + fly_dir * speed;
    let dir: [f64; 3] = dir.try_into().unwrap();
    let pos: [f64; 3] = pos.try_into().unwrap();
    let velocity: [f64; 3] = velocity.try_into().unwrap();
    particles.push(Particle::new(pos, velocity, mass, density));
  }
}
