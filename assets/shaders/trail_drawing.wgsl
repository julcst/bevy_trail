// #import bevy_pbr::mesh_view_bindings::view

struct VertexIn {
    @builtin(vertex_index) index: u32,
};

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
};

struct Trail {
    head: u32,
    length: u32,
    capacity: u32,
};

struct TrailPoint {
    position: vec3f,
    width: f32,
    color: vec4f,
    velocity: vec3f,
    length: f32,
};

struct TrailStyle {
    taper: f32,
    fade: f32,
    profile: u32,
};

@group(0) @binding(0) var<uniform> trail: Trail;
@group(0) @binding(1) var<storage, read> trail_points: array<TrailPoint>;
@group(0) @binding(2) var<uniform> style: TrailStyle;

fn create_vert(x: f32, y: f32) -> VertexOut {
    return VertexOut(vec4<f32>(x, y, 0.0, 1.0));
}

@vertex
fn vertex(in: VertexIn) -> VertexOut {
    let side = in.index % 2u == 0u;
    let idx = in.index / 2u;
    let point = trail_points[idx];
    return create_vert(point.position.x, point.position.y + select(-0.1, 0.1, side));
}

@fragment
fn fragment(in: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
