const G: f32 = 6.67408E-11;
const PARTICLES_PER_GROUP: u32 = 256; // REMEMBER TO CHANGE MAIN.RS

struct Particle {
    /// Position
    pos: vec3<f32>, // 4, 8, 12

    /// The radius of the particle (currently unused)
    radius: f32, // 16

    /// Velocity
    vel: vec3<f32>, // 4, 8, 12
    _p: f32, // 16

    /// Mass
    mass: f64, // 4, 8
    _p2: vec2<f32>, // 12, 16
}


struct SimParams {
  deltaT : f32,
  rule1Distance : f32,
  rule2Distance : f32,
  rule3Distance : f32,
  rule1Scale : f32,
  rule2Scale : f32,
  rule3Scale : f32,
  safety : f32,
};

fn length2(v: vec3<f32>) {
    return v.x * v.x + v.y * v.y + v.z * v.z;
}


@group(0) @binding(0) var<uniform> params : SimParams;
@group(0) @binding(1) var<storage, read> particlesSrc : array<Particle>;
@group(0) @binding(2) var<storage, read_write> particlesDst : array<Particle>;
@group(0) @binding(3) var<storage, read> data_old: array<Particle>;
@group(0) @binding(4) var<storage, read> data: array<Particle>;

// https://github.com/austinEng/Project6-Vulkan-Flocking/blob/master/data/shaders/computeparticles/particle.comp
@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
  let total = arrayLength(&particlesSrc);
  let index = global_invocation_id.x;
  if (index >= total) {
    return;
  }
    // Early return
    if(data_old[i].mass < 0) { 
        return;
    }

    // Gravity
    if(delta > 0.0) {
        let temp: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);

        // Go through all other particles...
        for(let j = 0; j < particles; j++) {
            // Skip self
            if(j == i) { continue; }

            // If a single particle with no mass is encountered, the entire loop
            // terminates (because they are sorted by mass)
            if(data_old[j].mass == 0) { break; }

            let diff: vec3<f32> = data_old[j].pos - data_old[i].pos;
            temp += normalize(diff) * data_old[j].mass / (length2(diff)+safety);
        }

        // Update data
        data[i].vel += vec3(temp * G * delta);
        data[i].pos += data[i].vel * delta;
    }
}