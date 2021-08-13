// Vertex shader

let vertices: array<vec2<f32>, 3> = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, 1.0),
    vec2<f32>(3.0, 1.0),
    vec2<f32>(-1.0, -3.0),
);

let MAX_STEPS:u32 = 100u;
let MAX_DIST:f32 = 100.0;
let SURF_DIST:f32 = 0.01;

struct VertexOutput {
    [[location(0)]] coord: vec2<f32>;
    [[builtin(position)]] clip_position: vec4<f32>;
};

[[stage(vertex)]]
fn main([[builtin(vertex_index)]] in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    out.coord = vertices[in_vertex_index];
    out.clip_position = vec4<f32>(out.coord, 0.0, 1.0);
    return out;
}

// Fragment shader

// distance between plane ans sphare of exis aligned 
fn GetDist(p:vec3<f32>)->f32 {
    let sphere = vec4<f32>(0.0, 1.0, 6.0, 1.0);
    let dS = length(p-sphere.xyz)-sphere.w;
    let dP = p.y;
    let d = min(dS, dP);
    return d;
}

fn RayMarch(ro:vec3<f32>, rd:vec3<f32>)->f32 {
    var dO:f32 = 0.0;
    for (var i:u32=0u; i<MAX_STEPS; i=i+1u) {
        let p = ro + dO*rd;
        let dS = GetDist(p);
        dO = dO + dS;
        if (dO>MAX_DIST || dS<SURF_DIST) { break };
    }
    return dO;
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let uv = (in.coord + vec2<f32>(1., 1.)) / 2.;
    //let uv = (in.coord - (0.5*in.coord)) / in.coord.y;

    let ro = vec3<f32>(0.0, 1.0, 0.0); // ray/camera origin
    let rd = normalize(vec3<f32>(uv.x, uv.y, 1.0)); // ray/camera direction

    let d = RayMarch(ro, rd);
    let col = vec3<f32>(d/6.0);

    return vec4<f32>(col, 1.0);
}
