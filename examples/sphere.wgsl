[[block]]
struct Uniforms {
    resolution: vec2<f32>; // in pixels
    playtime: f32; // in seconds
};

[[group(0), binding(0)]]
var uniforms: Uniforms;

struct VertexOutput {
    [[location(0)]] coord: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

let MAX_STEPS:u32 = 100u;
let MAX_DIST:f32 = 100.0;
let SURF_DIST:f32 = 0.01;

// distance between plane ans sphare when axis align
fn GetDist(p:vec3<f32>)->f32 {
    let sphere = vec4<f32>(0.0, 1.0, 6.0, 1.0);
    let sphereDist = length(p-sphere.xyz)-sphere.w;
    let planeDist = p.y;
    return min(sphereDist, planeDist);
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

fn GetNormal(p:vec3<f32>)->vec3<f32> {
    let d = GetDist(p);
    let n = d - vec3<f32>(
        GetDist(p-vec3<f32>(0.01, 0.0, 0.0)),
        GetDist(p-vec3<f32>(0.0, 0.01, 0.0)),
        GetDist(p-vec3<f32>(0.0, 0.0, 0.01)));
    return normalize(n);
}

fn GetLight(p: vec3<f32>)->f32 {
    var lightPos:vec3<f32> = vec3<f32>(0.0, 5.0, 6.0);
    lightPos = lightPos + vec3<f32>(sin(uniforms.playtime), cos(uniforms.playtime), 0.0) * 2.0;
    let l = normalize(lightPos-p);
    let n = GetNormal(p)*SURF_DIST*2.0;
    var dif:f32 = clamp(dot(n, l), 0.0, 1.0);
    let d = RayMarch(p + n, l);
    if (d<length(lightPos-p)) { dif = dif * 0.1; }
    return dif;
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let uv = ((in.position.xy - 0.5*uniforms.resolution) / uniforms.resolution.y)*vec2<f32>(1.0,-1.0);
    let ro = vec3<f32>(0.0, 1.0, 0.0); // ray/camera origin
    let rd = normalize(vec3<f32>(uv.x, uv.y, 1.0)); // ray/camera direction
    let d = RayMarch(ro, rd);
    let p = ro + rd * d;
    let difuse = GetLight(p);
    var col:vec3<f32> = vec3<f32>(difuse);
    col = pow(col, vec3<f32>(0.4545));
    return vec4<f32>(col, 1.0);
}

