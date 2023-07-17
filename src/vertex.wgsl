[[stage(vertex)]]
fn main(
    [[builtin(vertex_index)]] vertexIndex : u32,
    [[builtin(position)]] position : vec4<f32>,
    [[location(0)]] var<out> fragColor : vec3<f32>,
    [[builtin(point_size)]] var<out> pointSize : f32
) -> [[builtin(position)]] vec4<f32> {
    var i : i32 = i32(vertexIndex);

    if (dataCurrent.data[uint(i)].mass < 0.0) {
        pointSize = 0.0;
        return position;
    }

    var worldPosition : vec4<f32> = globalsBuffer.matrix * vec4<f32>(dataCurrent.data[uint(i)].pos, 1.0);
    pointSize = clamp(30.0 * 1E11 / worldPosition.z, 1.0, 20.0);

    if (dataCurrent.data[uint(i)].mass > 1E33) {
        fragColor = vec3<f32>(0.0, 0.0, 0.0);
    } else {
        if (i < i32(globalsBuffer.particles) / 2 + 1) {
            fragColor = vec3<f32>(0.722, 0.22, 0.231);
        } else {
            fragColor = vec3<f32>(0.345, 0.522, 0.635);
        }
    }

    return worldPosition;
}
