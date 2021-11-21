[[block]] struct Uniforms {
  resolution: vec2<f32>; // in pixels
  playtime: f32; // in seconds
};

[[group(0), binding(0)]] var<uniform> uniforms: Uniforms;

struct VertexOutput {
  [[location(0)]] coord: vec2<f32>;
  [[builtin(position)]] position: vec4<f32>;
};

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
  let center = 0.5*uniforms.resolution.xy;
  let uv = ((in.position.xy - center) / uniforms.resolution.y) * 1.0;
  let bg = vec4<f32>(0.0,0.0,1.0,1.0);
  let red = vec4<f32>(1.0,0.0,0.0,1.0);
  let radius = 0.25;
  let l = length(uv)-radius;
  let fg = vec4<f32>(red.xyz, floor(1.0 - l));
  return mix(bg, fg, fg.a);
}