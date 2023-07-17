use {
    crate::Particle,
    cgmath::{
        prelude::*,
        {Point3, Vector3},
    },
    rand::prelude::*,
    std::f32::consts::PI,
};

const G: f64 = 6.67408e-11;
const ARMS: u32 = 4;

pub fn create(
    angle: f32,
    normal: Vector3<f32>,
    particles: &mut Vec<Particle>,
    calibrate: f64,
    center_pos: Point3<f32>,
    center_vel: Vector3<f32>,
    center_mass: f64,
    radius: f32,
) {
    let tangent = normal.cross(Vector3::new(normal.z, normal.y, normal.x));
    let diff = tangent * angle.sin() + normal.cross(tangent) * angle.cos();
    let movement = diff.cross(normal).normalize();
    let pos = center_pos + diff * radius;
    let speed =
        (G * center_mass * radius as f64 / ((radius * radius) as f64 + calibrate)).sqrt() as f32;
    let vel = center_vel + movement * speed;
    particles.push(Particle::new(pos.into(), vel.into(), 0.0, calibrate));
}

pub fn formation(
    particles: &mut Vec<Particle>,
    amount: u32,
    calibrate: f64,
    center_pos: Point3<f32>,
    center_vel: Vector3<f32>,
    center_mass: f64,
    normal: Vector3<f32>,
) {
    for _ in 0..amount / 5 {
        let radius = 5e9
            + (rand_distr::Normal::<f32>::new(0.0, 1e11)
                .unwrap()
                .sample(&mut thread_rng()))
            .abs();
        let angle = thread_rng().gen::<f32>() * 2.0 * PI;
        create(
            angle,
            normal.normalize(),
            particles,
            calibrate,
            center_pos,
            center_vel,
            center_mass,
            radius,
        );
    }

    // based on number of stars in the arms vs center of Milky Way
    for _ in 0..amount / 5 * 4 {
        let radius = 5e9
            + (rand_distr::Normal::<f32>::new(0.0, 1e11)
                .unwrap()
                .sample(&mut thread_rng()))
            .abs();
        let arm = rand_distr::Uniform::from(0..ARMS).sample(&mut thread_rng());
        let angle = (arm as f32 / ARMS as f32 * 2.0 * PI) - (radius * 1e-11)
            + rand_distr::Normal::new(0.0, PI / 16.0)
                .unwrap()
                .sample(&mut thread_rng());
        create(
            angle,
            normal.normalize(),
            particles,
            calibrate,
            center_pos,
            center_vel,
            center_mass,
            radius,
        );
    }
}
