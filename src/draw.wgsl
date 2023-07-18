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

struct DataCurrent {
    data : array<Particle>,
};

struct vertexOutput {
    @builtin(position) pos : vec4<f32>,
    @location(0) fragColor : vec3<f32>,
    @location(1) pointSize : f32,
};


@group(0) @binding(0) var<uniform> gpu_info : Gpu_Info;
@group(0) @binding(1) var<storage, read_write> dataCurrent : DataCurrent;
@vertex
fn vs_main(
    @builtin(vertexIndex) vertexIndex : u32,
    @location(1) position : vec4<f32>,
    @location(2) fragColor : vec3<f32>,
    @location(3) pointSize : f32
) {
    let i : i32 = i32(vertexIndex);

    if (dataCurrent.data[i].mass < f64(0.0)) {
        pointSize = 0.0;
        return position;
    }

    let glpos : vec4<f32> = gpu_info.matrix * vec4<f32>(dataCurrent.data[i].pos, 1.0);
    pointSize = clamp(30.0 * 1E11 / glpos.z, 1.0, 20.0);

    if (dataCurrent.data[i].mass > f64(1E33)) {
        fragColor = vec3<f32>(0.0, 0.0, 0.0);
    } else {
        if (i < i32(gpu_info.particles) / 2 + 1) {
            fragColor = vec3<f32>(0.722, 0.22, 0.231);
        } else {
            fragColor = vec3<f32>(0.345, 0.522, 0.635);
        }
    }

    return glpos;
}