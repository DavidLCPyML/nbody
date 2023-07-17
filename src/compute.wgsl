type Particle = {
    pos : vec3<f32>;
    _p1 : f32;
    vel : vec3<f32>;
    _pad : f32;
    mass : f64;
    calibrate : f64;
};

[[block]]
struct GlobalsBuffer {
    matrix : mat4x4<f32>;
    particles : u32;
    delta : f32;
    _pad : vec2<f32>;
};

[[block]]
struct DataOld {
    old : [[stride(32)]] array<Particle>;
};

[[block]]
struct DataCurrent {
    data : [[stride(32)]] array<Particle>;
};

[[binding(0), group(0)]] var<uniform> globalsBuffer : GlobalsBuffer;
[[binding(1), group(0)]] var<DataOld> dataOld : DataOld;
[[binding(2), group(0)]] var<DataCurrent> dataCurrent : DataCurrent;

fn length2(v : vec3<f64>) -> f64 {
    return v.x * v.x + v.y * v.y + v.z * v.z;
}

[[stage(compute), workgroup_size(PARTICLES_PER_GROUP), binding(0, 0)]]
fn main([[builtin(global_invocation_id)]] globalInvocationID : vec3<u32>) {
    let i : u32 = globalInvocationID.x;

    if (dataOld.old[i].mass < 0.0) {
        return;
    }

    // Gravity
    if (globalsBuffer.delta > 0.0) {
        var temp : vec3<f64> = vec3<f64>(0.0, 0.0, 0.0);
        for (var j : i32 = 0; j < i32(globalsBuffer.particles); j = j + 1) {
            if (j == i) {
                continue;
            }
            if (dataOld.old[uint(j)].mass == 0.0) {
                break;
            }

            var diff : vec3<f64> = dataOld.old[uint(j)].pos - dataOld.old[i].pos;
            temp = temp + normalize(diff) * f64(dataOld.old[uint(j)].mass) / (length2(diff) + dataOld.old[uint(j)].calibrate);
        }
        dataCurrent.data[i].vel = dataCurrent.data[i].vel + vec3<f32>(temp * f64(globalsBuffer.delta) * G);
        dataCurrent.data[i].pos = dataCurrent.data[i].pos + dataCurrent.data[i].vel * f32(globalsBuffer.delta);
    }
}
