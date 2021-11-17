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

let ITERATIONS: i32 = 50;

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let c:vec2<f32> = (in.coord + vec2<f32>(-0.5, 0.)) * 1.3;
    var x:f32 = 0.0;
    var y:f32 = 0.0;
    var i:i32 = 0;
    
    for (; i < ITERATIONS; i = i + 1) {
        if (x*x + y*y > 4.) { break; }
        let xtemp:f32 = (x * x) - (y * y) + c.x;
        y = 2.0 * x * y + c.y;
        x = xtemp;
    }

    let frac:f32 = f32(i) / f32(ITERATIONS);
    return vec4<f32>(1.0-(1.0*frac), 1.0-(1.0*frac), 1.0-(5.0*frac), 1.0);
}
