[[block]]
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

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
  let uv = ((in.position.xy - 0.5*uniforms.resolution) / min(uniforms.resolution.x, uniforms.resolution.y))*vec2<f32>(1.0,-1.0);
  let col = vec3<f32>(uv.x, uv.y, fract(uv.x*uv.y));
  return vec4<f32>(col, 1.0);
}
