use nbody::run;
pub use nbody::state::gen;

use cgmath::{
    Point3,
    Vector3
};

fn defaults() -> gen::Galaxies {
    gen::Galaxies {
    galaxy: vec![
        gen::Galaxy::Structures {
            center_pos: [1e9, 1e9, 0.0],
            center_velocity: [1e5, 0.0, 0.0],
            center_mass: 1e30,
            num_particles: 10000,
            normal: [1.0, 0.0, 0.0],
            center_density: 1e20,
        },
        gen::Galaxy::Structures {
            center_pos: [-1e9, -1e9, 0.0],
            center_velocity: [0.0, 0.0, 0.0],
            center_mass: 1e30,
            num_particles: 10000,
            normal: [1.0, 1.0, 0.0],
            center_density: 1e20,

        },
    ],
}
}

fn main() {
    let default: gen::Galaxies = defaults();
    let particles = default.new();

    let globals = gen::Globals::default();

    pollster::block_on(run(globals, particles));
}

