use {
    crate::Particle,
    cgmath::{
        prelude::*,
        {Point3, Vector3},
    },
    rand::{thread_rng, Rng},
    std::f32::consts::PI,
};

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
    // normalize(cross(N, T')), T' is arbitrary vector
    let tangent: Vector3<f32> = normal.cross(Vector3::new(normal.z, normal.y, normal.x));
    // cross(N, T) for movement
    let orthogonal: Vector3<f32> = tangent * angle.sin() + normal.cross(tangent) * angle.cos();
    let movement: Vector3<f32> = orthogonal.cross(normal).normalize();
    // is radius really necessary?
    // pos = center + offset
    let pos: Point3<f32> = center_pos + orthogonal * radius;
    let gravity: f64 = 6.67408e-11;
    // gravitational acceleration formula
    // see if you can get rid of the calibrate value and if it still works well
    let speed: f32 = (gravity * center_mass * radius as f64
        / ((radius * radius) as f64 + calibrate))
        .sqrt() as f32;
    // V' = V+g, g = gravitational acceleration * vector of movement
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
        let radius = 5e9 + thread_rng().gen_range(0.0..1e11);
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

    // based on number of stars in the arms vs center of Milky Way (80%)
    for _ in 0..(amount * (4 / 5)) {
        let arms = 4;
        let radius = 5e9 + thread_rng().gen_range(0.0..1e11);
        // θ = (2π / n) + (2π / n_arm) * (arm_number - 1) + f(r)
        // f(r) is a function that includes variation in the number
        let arm: f32 = thread_rng().gen_range(0..(arms)) as f32;
        let angle = (arm as f32 / (arms as f32) * 2.0 * PI) - (radius * 1e-11)
            + thread_rng().gen_range(0.0..0.15);
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
