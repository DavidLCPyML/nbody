struct VertexOut {
    @location(0) fragColor : vec3<f32>,
    @builtin(position) pos : vec4<f32>,
    @location(1) pointSize : f32,
};

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
  return vec4(in.fragColor, 1.0);
}