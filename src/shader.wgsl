// Vertex shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
};


@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    var out: VertexOutput;
    out.color = model.color;
    out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
    return out;
}


const BLUE: vec4<f32> = vec4<f32>(0.0, 0.0, 0.635, 1.0);
const RED: vec4<f32> = vec4<f32>(0.635, 0.0, 0.0, 1.0);

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return RED;
}


const G: f32 = 6.67408E-11;
const PARTICLES_PER_GROUP: i32 = 256; // REMEMBER TO CHANGE MAIN.RS

struct GlobalsBuffer {
    num_particles: i32,
    safety: f32,
    delta: f32,
};

struct Particle {
    p: vec3<f64>,
    v: vec3<f64>,
    mass: f64,
    density: f64,
}

struct DataOld {
    data_old: array<Particle>,
};

struct DataCurrent {
    data: array<Particle>,
};

fn length2(v: vec3<f64>) -> f32 {
    return v.x * v.x + v.y * v.y + v.z * v.z;
}

@compute
fn cs_main(@builtin(global_invocation_id) globalInvocationID: vec3<u32>) {
    // Get index of current particle
    var i = globalInvocationID.x;

    // Early return
    if(data_old[i].mass < 0) { 
        return;
    }

    // Gravity
    if(delta > 0.0) {
        var temp = dvec3(0.0, 0.0, 0.0);

        // Go through all other particles...
        for(var j = 0; j < particles; j++) {
            // Skip self
            if(j == i) { continue; }

            if(data_old[j].mass == 0) { break; }

            var diff = data_old[j].pos - data_old[i].pos;
            temp += normalize(diff) * data_old[j].mass / (length2(diff)+safety);
        }

        // Update data
        data[i].vel += vec3(temp * G * delta);
        data[i].pos += data[i].vel * delta;
    }
}