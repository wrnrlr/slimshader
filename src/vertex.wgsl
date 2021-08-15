var<private> vertices: array<vec2<f32>, 3> = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, 1.0),
    vec2<f32>(3.0, 1.0),
    vec2<f32>(-1.0, -3.0),
);

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] coord: vec2<f32>;
};

[[stage(vertex)]]
fn main([[builtin(vertex_index)]] in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    out.coord = vertices[in_vertex_index];
    out.clip_position = vec4<f32>(out.coord, 0.0, 1.0);
    return out;
}
