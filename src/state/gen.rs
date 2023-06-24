use cgmath::{
    prelude::*,
    {Point3, Vector3},
};
use std::f32::consts::PI;
use rand::prelude::*;

const G: f64 = 6.67408E-11;
const ARMS: u32 = 4;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Particle {
  pos: Point3<f64>,
  velocity: Vector3<f64>,
  pub mass: f64,
  pub density: f64,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Globals {
    camera_pos: Point3<f32>, // 1, 2, 3
    particles: u32, // 4
    safety: f64, // 5, 6
    delta: f32, // 7
    _p: f32, // 8
}

impl Particle {
    pub fn new(pos: Point3<f64>, velocity: Vector3<f64>, mass: f64, density: f64) -> Self {
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
    pos: Point3<f64>,
    velocity: Vector3<f64>,
    mass: f64,
    density: f64,
  },
  Structures {
    num_particles: u32,
    center_pos: Point3<f64>,
    center_mass: f64,
    center_velocity: Vector3<f64>,
    center_density: f64,
    normal: Vector3<f64>,
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
          *center_pos,
          *center_mass,
          *center_velocity,
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
    center_pos: Point3<f64>,
    center_mass: f64,
    center_velocity: Vector3<f64>,
    center_density: f64,
    mut normal: Vector3<f64>,
) {
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
    particles.push(Particle::new(pos, velocity, mass, density));
  }
}
