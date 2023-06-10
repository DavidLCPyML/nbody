use cgmath::{
    prelude::*,
    {Point3, Vector3},
};

const G: f64 = 6.67408E-11;
const ARMS: u32 = 4;

pub struct Particle {
    pos: Point3<f32>,
    velocity: Vector3<f32>,
    mass: f64,
    density: f64,
}

impl Particle {
    fn new(pos: Point3<f32>, velocity: Vector3<f32>, mass: f64, density: f64) -> Self {
        Self {
            pos,
            velocity,
            mass,
            density,
        }
    }
}

pub fn create_galaxy(
    particles: &mut Vec<Particle>,
    num_particles: u32,
    center_pos: Point3<f32>,
    center_mass: f64,
    center_velocity: Vector3<f32>,
    center_density: f64,
    mass: f64,
    density: f64,
    mut normal: Vector3<f32>,
) {
  normal = normal.normalize();
  let tangent: Vector3<f32> = normal.cross(Vector3::new(normal.y, -normal.x, normal.z));
  let bitangent = normal.cross(tangent);
  let radius: f64 = 3E12 as f64;

  for _ in 0..num_particles / 5 {
    let theta: f32 = thread_rng().gen::<f32>() * 2.0 * PI;
    let dir: f32 = tangent * cos(theta) + bitangent * sin(theta);
    let pos = center_pos + dir * radius * cos(theta);
    let speed = (G * center_mass / radius).sqrt();
    let fly_dir = dir.cross(normal); // check if necessary
    let velocity = center_velocity + fly_dir * speed;
    particles.push(Particle::new(pos, velocity, mass, density));
  }

  for _ in 0..num_particles / 5 * 4 {
    let arm = thread_rng().gen_range::<u32>(0, ARMS);

    let theta: f32 = arm / ARMS * 2.0 * PI;
    let dir: f32 = tangent * cos(theta) + bitangent * sin(theta);
    let pos = center_pos + dir * radius * cos(theta);
    let speed = (G * center_mass / radius).sqrt();
    let fly_dir = dir.cross(normal); // check if necessary
    let velocity = center_velocity + fly_dir * speed;
    particles.push(Particle::new(pos, velocity, mass, density));
  }
}
