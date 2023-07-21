struct Particle {
    pos : vec3<f32>,
    _pad1 : f32,
    vel : vec3<f32>,
    _pad : f32,
    mass : f32,
    calibrate : f32,
    _pad2 : vec2<f32>,
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

struct VertexIn {
    @builtin(vertex_index) vertexIndex : u32,
};
struct VertexOut {
    @location(0) fragColor : vec3<f32>,
    @builtin(position) pos : vec4<f32>,
    @location(1) pointSize : f32,
};

@group(0) @binding(0) var<uniform> gpu_info : Gpu_Info;
@group(0) @binding(2) var<storage, read_write> dataCurrent : DataCurrent;
@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    let i : i32 = i32(input.vertexIndex);
    var output: VertexOut;
    if (dataCurrent.data[i].mass < 0.0) {
        output.pos = vec4<f32> (dataCurrent.data[i].pos, 1.0);
    }
    output.pos = gpu_info.matrix * vec4<f32>(dataCurrent.data[i].pos, 1.0);

    if (dataCurrent.data[i].mass > 1E33) {
        output.fragColor = vec3<f32>(0.0, 0.0, 0.0);
    } else {
        if (i < i32(gpu_info.particles) / 2 + 1) {
            output.fragColor = vec3<f32>(1.0, 0.5, 0.67);
        } else {
            output.fragColor = vec3<f32>(0.4, 0.5, 1.0);
        }
    }
    return output;
}
