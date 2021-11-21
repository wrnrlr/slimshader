// Defaults ///////////////////////////////////////////////

let Replace:i32 = 0; // default: replace the old source with the new one
let Alpha:i32 = 1; // alpha-blend the new source on top of the old one
let Multiply:i32 = 2; // multiply the new source with the old one
let DEFAULT_SHAPE_V = 1e+20;
let DEFAULT_CLIP_V = -1e+20;
let DEFAULT_DEPTH = 1e+30;

// Globals ////////////////////////////////////////////////

var<private> aspect:vec2<f32>;
var<private> uv:vec2<f32>;
// var<private> position:vec2<f32>;
// var<private> query_position:vec2<f32>;
var<private> ScreenH:f32;
var<private> AA:f32;
var<private> AAINV:f32;

// Utils //////////////////////////////////////////////////

fn det(a:vec2<f32>, b:vec2<f32>)->f32 {
  return a.x*b.y-b.x*a.y;
}

fn length2(a:vec4<f32>)->vec2<f32> {
  return vec2<f32>(length(a.xy),length(a.zw));
}

fn dot2(a:vec4<f32>,b:vec2<f32>)->vec2<f32> {
  return vec2<f32>(dot(a.xy,b), dot(a.zw,b));
}

// Stack Context //////////////////////////////////////////

struct Ctx {
  position:vec4<f32>;
  shape:vec2<f32>;
  clip:vec2<f32>;
  scale:vec2<f32>;
  line_width:f32;
  premultiply:bool;
  depth_test:bool;
  blur:vec2<f32>;
  source:vec4<f32>;
  start_pt:vec2<f32>;
  last_pt:vec2<f32>;
  source_blend:i32;
  has_clip:bool;
  source_z:f32;
};

var<private> STACK:Ctx;

fn init(fragCoord:vec2<f32>, resolution:vec2<f32>) {
  aspect = vec2<f32>(resolution.x / resolution.y, 1.0);
	ScreenH = min(resolution.x, resolution.y);
	AA = ScreenH*0.4;
	AAINV = 1.0 / AA;
    
  uv = fragCoord.xy / resolution;
//   // vec2 m = mouse / resolution;

  let position = (uv*2.0-(1.0))*aspect * vec2<f32>(1.0,-1.0);
//   // query_position = (m*2.0-1.0)*aspect;

  STACK = Ctx(
    vec4<f32>(position.x, position.y, position.x, position.y), //position, query_position
    vec2<f32>(DEFAULT_SHAPE_V),
    vec2<f32>(DEFAULT_CLIP_V),
    vec2<f32>(1.0,1.0),
    1.0,
    false,
    false,
    vec2<f32>(0.0,1.0),
    vec4<f32>(0.0,0.0,0.0,1.0),
    vec2<f32>(0.0),
    vec2<f32>(0.0),
    Replace,
    false,
    DEFAULT_DEPTH
  );
}

var<private> COLOR:vec3<f32> = vec3<f32>(1.0,1.0,1.0);
var<private> DEPTH:f32 = DEFAULT_DEPTH;

fn add_clip(d:vec2<f32>) {
  let d = d / STACK.scale;
  STACK.clip = max(STACK.clip, d);
  STACK.has_clip = true;
}

fn add_field(d:vec2<f32>) {
  let d = d / STACK.scale;
  STACK.shape = min(STACK.shape, d);
}

// Color ops //////////////////////////////////////////////

fn blit()->vec4<f32> {
  return vec4<f32>(pow(COLOR.rgb, vec3<f32>(1.0/2.2)), 1.0);
}

fn clear() {
  COLOR = mix(COLOR.rgb, STACK.source.rgb, STACK.source.a);
  if (STACK.source.a == 1.0) {
    DEPTH = STACK.source_z;
  }
}

fn new_path() {
  STACK.shape = vec2<f32>(DEFAULT_SHAPE_V);
  STACK.clip = vec2<f32>(DEFAULT_CLIP_V);
  STACK.has_clip = false;
}

fn write_color(rgba:vec4<f32>, w:f32) {
  if (STACK.depth_test) {
    if (w == 1.0 && STACK.source_z <= DEPTH) {
      DEPTH = STACK.source_z;
    } else {
      if (w == 0.0 || STACK.source_z > DEPTH) { return; }
    }
  }

  let src_a = w * rgba.a;
  var dst_a:f32 = w;
  if (STACK.premultiply != true) {
    dst_a = src_a;
  }
  COLOR = COLOR.rgb * (1.0 - src_a) + rgba.rgb * dst_a;
}

fn min_uniform_scale()->f32 {
    return min(STACK.scale.x, STACK.scale.y);
}

fn uniform_scale_for_aa()->f32 {
    return min(1.0, STACK.scale.x / STACK.scale.y);
}

fn calc_aa_blur(w:f32)->f32 {
  let blur = STACK.blur;
  let w = w - blur.x;
  let wa = clamp(-w*AA*uniform_scale_for_aa(), 0.0, 1.0);
  let wb = clamp(-w / blur.x + blur.y, 0.0, 1.0);
	return wa * wb;
}

fn fill_preserve() {
  write_color(STACK.source, calc_aa_blur(STACK.shape.x));
  if (STACK.has_clip) {
    write_color(STACK.source, calc_aa_blur(STACK.clip.x));        
  }
}

fn fill() {
  fill_preserve();
  new_path();
}

fn set_line_width(w:f32) {
    STACK.line_width = w;
}

fn set_line_width_px(w:f32) {
    STACK.line_width = w * min_uniform_scale() * AAINV;
}

fn get_gradient_eps()->f32 {
    return (1.0 / min_uniform_scale()) * AAINV;
}

fn stroke_shape()->vec2<f32> {
  return abs(STACK.shape) - STACK.line_width/STACK.scale;
}

fn stroke_preserve() {
  let w = stroke_shape().x;
  write_color(STACK.source, calc_aa_blur(w));
}

fn stroke() {
  stroke_preserve();
  new_path();
}

fn set_source_rgba(c:vec4<f32>) {
  // c.rgb = c.rgb;
  let c2 = c * c;
  if (STACK.source_blend == Multiply) {
      STACK.source = STACK.source * c2;
  } else {
    if (STACK.source_blend == Alpha) {
      let src_a = c2.a;
      var dst_a:f32;
      if (STACK.premultiply) { dst_a = 1.0; } else { dst_a = src_a; }
      STACK.source = vec4<f32>(STACK.source.rgb * (1.0 - src_a) + c2.rgb * dst_a,
                    max(STACK.source.a, c2.a));
    } else {
    	STACK.source = c2;
    }
  }
}

// Save current stroke width, starting point and blend mode from active context.
fn save()->Ctx {
  return STACK;
}

// // Restore stroke width, starting point and blend mode to a context previously returned by save()
fn restore(ctx:Ctx) {
  // preserve shape & source
  let shape = STACK.shape;
  let clip = STACK.clip;
  let has_clip = STACK.has_clip;
  let source = STACK.source;
  STACK = ctx;
  STACK.shape = shape;
  STACK.clip = clip;
  STACK.source = source;
  STACK.has_clip = has_clip;
}

fn move_to(p:vec2<f32>) {
  STACK.start_pt = p;
  STACK.last_pt = p;
}

fn line_to(p:vec2<f32>) {
  let pa = STACK.position - STACK.last_pt.xyxy;
  let ba = p - STACK.last_pt;
  let h = clamp(dot2(pa,ba) / dot(ba,ba), vec2<f32>(0.0), vec2<f32>(1.0));
  let s = sign(pa.xz*ba.y-pa.yw*ba.x);
  let d = length2(pa - ba.xyxy*h.xxyy);
  add_field(d);
  add_clip(d * s);
  STACK.last_pt = p;
}

// Bezier /////////////////////////////////////////////////
// from https://www.shadertoy.com/view/ltXSDB

// Test if point p crosses line (a, b), returns sign of result
fn test_cross(a:vec2<f32>, b:vec2<f32>, p:vec2<f32>)->f32 {
  return sign((b.y-a.y) * (p.x-a.x) - (b.x-a.x) * (p.y-a.y));
}

// // Determine which side we're on (using barycentric parameterization)
fn bezier_sign(A:vec2<f32>, B:vec2<f32>, C:vec2<f32>, p:vec2<f32>)->f32 {
  let a = C - A; let b = B - A; let c = p - A;
  let bary = vec2<f32>(c.x*b.y-b.x*c.y,a.x*c.y-c.x*a.y) / (a.x*b.y-b.x*a.y);
  let d = vec2<f32>(bary.y * 0.5, 0.0) + 1.0 - bary.x - bary.y;
  return mix(sign(d.x * d.x - d.y), mix(-1.0, 1.0,
    step(test_cross(A,B,p) * test_cross(B, C, p), 0.0)),
    step((d.x - d.y), 0.0)) * test_cross(A,C,B);
} 

// Solve cubic equation for roots
fn bezier_solve(a:f32, b:f32, c:f32)->vec3<f32> {
  let p = b - a*a / 3.0;
  let p3 = p*p*p;
  let q = a * (2.0*a*a - 9.0*b) / 27.0 + c;
  let d = q*q + 4.0*p3 / 27.0;
  let offset = -a / 3.0;
  if (d >= 0.0) {
    let z = sqrt(d);
    let x = (vec2<f32>(z, -z) - q) / 2.0;
    let uv = sign(x) * pow(abs(x), vec2<f32>(1.0/3.0));
    return vec3<f32>(offset + uv.x + uv.y);
  }
  let v = acos(-sqrt(-27.0 / p3) * q / 2.0) / 3.0;
  let m = cos(v);
  let n = sin(v)*1.732050808;
  return vec3<f32>(m + m, -n - m, n - m) * sqrt(-p / 3.0) + offset;
}

// // Find the signed distance from a point to a quadratic bezier curve
fn bezier(A:vec2<f32>, B:vec2<f32>, C:vec2<f32>, p:vec2<f32>)->f32 {
  let BB = mix(B + vec2<f32>(1e-4,1e-4), B, abs(sign(B*2.0 - A - C)));
  let a = BB - A;
  let b = A - BB * 2.0 + C;
  let c = a * 2.0;
  let d = A - p;
  let k = vec3<f32>(vec2<f32>(3.0,3.0) * dot(a,a) + dot(d,b), dot(d,a)) / dot(b,b);
  let t = clamp(bezier_solve(k.x, k.y, k.z), vec3<f32>(0.0,0.0,0.0), vec3<f32>(1.0,1.0,1.0));
  var pos = A + (c + b*t.x)*t.x;
  var dis = length(pos - p);
  pos = A + (c + b*t.y)*t.y;
  dis = min(dis, length(pos - p));
  pos = A + (c + b*t.z)*t.z;
  dis = min(dis, length(pos - p));
  return dis * bezier_sign(A, B, C, p);
}

fn curve_to(b1:vec2<f32>, b2:vec2<f32>) {
  let shape = vec2<f32>(
    bezier(STACK.last_pt, b1, b2, STACK.position.xy),
    bezier(STACK.last_pt, b1, b2, STACK.position.zw));
  add_field(abs(shape));
  add_clip(shape);
  STACK.last_pt = b2;
}

// Main ///////////////////////////////////////////////////

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

fn shield_shape() {
  move_to(vec2<f32>(0.2, 0.2));
  line_to(vec2<f32>(0.0, 0.3));
  line_to(vec2<f32>(-0.2, 0.2));    
  curve_to(vec2<f32>(-0.2, -0.05), vec2<f32>(0.0, -0.2));
  curve_to(vec2<f32>(0.2, -0.05), vec2<f32>(0.2, 0.2));
}

fn circle(uv:vec2<f32>, center:vec2<f32>, radius:f32) {
	let d = length(center - uv) - radius;
	let t = clamp(d, 0.0, 1.0);
  let l = length(uv)-radius;
	add_field(vec2<f32>(l,l));
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
  let aspect = uniforms.resolution.y / uniforms.resolution.x;
  let center = 0.5*uniforms.resolution.xy;
  let uv = ((in.position.xy - 0.5*uniforms.resolution) /
          min(uniforms.resolution.x, uniforms.resolution.y)) *
          vec2<f32>(1.0,-1.0);
  let blue = vec4<f32>(0.0,0.0,1.0,1.0);
  let red = vec4<f32>(1.0,0.0,0.0,1.0);
  init(in.position.xy, uniforms.resolution);
  set_source_rgba(blue);
  clear();
  set_source_rgba(red);
  let radius = 0.25;
  // circle(uv, center, radius);
  // fill();
  new_path();
  shield_shape();
  fill();
  return vec4<f32>(COLOR.rgb, 1.0);
}
