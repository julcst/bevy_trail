struct VertexIn {
    @builtin(vertex_index) index: u32,
};

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
};

fn create_vert(x: f32, y: f32) -> VertexOut {
    return VertexOut(vec4<f32>(x, y, 0.0, 1.0));
}

@vertex
fn vertex(in: VertexIn) -> VertexOut {
    let side = in.index % 2u == 0u;
    let idx = in.index / 2u;
    let y = select(-0.1, 0.1, side);
    let x = -0.5 + f32(idx) * 0.05;
    return create_vert(x, y + 0.5 * sin(x * 5.0));
}

@fragment
fn fragment(in: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
