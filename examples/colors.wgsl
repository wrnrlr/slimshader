struct Uniforms {
    resolution: vec2<f32>; // in pixels
    playtime: f32; // in seconds
};

[[group(0), binding(0)]]
var<uniform> uniforms: Uniforms;

struct VertexOutput {
    [[location(0)]] coord: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

fn sdSphere(p: vec3<f32>, r: f32) -> f32 {
  return length(p) - r;
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
  let uv = in.coord / uniforms.resolution;
  let col = 0.5 + 0.5*cos(uniforms.playtime+uv.xyx+vec3<f32>(0.0, 2.0, 4.0));
  return vec4<f32>(col, 1.0);
}