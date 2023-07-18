struct Particle {
    pos : vec3<f32>,
    _pad1 : f32,
    vel : vec3<f32>,
    _pad : f32,
    mass : f64,
    calibrate : f64,
};

struct Gpu_Info {
    matrix : mat4x4<f32>,
    particles : u32,
    motion : f32,
    _pad : vec2<f32>,
};

struct DataOld {
    old : array<Particle>,
};

struct DataCurrent {
    data : array<Particle>,
};

@group(0) @binding(0) var<uniform> gpu_info : Gpu_Info;
@group(0) @binding(1) var<storage, read> dataOld : DataOld;
@group(0) @binding(2) var<storage, read_write> dataCurrent : DataCurrent;

fn length2(v : vec3<f64>) -> f64 {
    return v.x * v.x + v.y * v.y + v.z * v.z;
}

// refactor workgroup?
@compute
@workgroup_size(256)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let i: u32 = global_invocation_id.x;
    let G: f64 = f64(6.67408e-11);

    if (dataOld.old[i].mass < f64(0.0)) {
        return;
    }

    // Gravity
    if (gpu_info.motion > 0.0) {
        var temp : vec3<f64> = vec3<f64>(vec3<f32>(0.0, 0.0, 0.0));
        for (var j : u32 = 0u; j < u32(gpu_info.particles); j = j + 1u) {
            if (j == i) {
                continue;
            }
            if (dataOld.old[j].mass == f64(0.0)) {
                break;
            }

            var diff : vec3<f64> = vec3<f64>(dataOld.old[j].pos - dataOld.old[i].pos);
            temp = temp + (normalize(diff) * (dataOld.old[j].mass) / (length2(diff) +
            dataOld.old[j].calibrate));
        }
        dataCurrent.data[i].vel = dataCurrent.data[i].vel + vec3<f32>(temp * G * f64(gpu_info.motion));
        dataCurrent.data[i].pos = dataCurrent.data[i].pos + dataCurrent.data[i].vel *
        gpu_info.motion;
    }
}
