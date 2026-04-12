// #import bevy_pbr::mesh_view_bindings::view

struct Header {
    head: u32,
    length: u32,
    capacity: u32,
};

struct TrailPoint {
    position: vec3f,
    width: f32,
    color: vec4f,
    velocity: vec3f,
    t: f32,
};

struct TrailStyle {
    taper: f32,
    fade: f32,
    profile: u32,
};

@group(0) @binding(0) var<uniform> header: Header;
@group(0) @binding(1) var<storage, read> data: array<TrailPoint>;
@group(0) @binding(2) var<uniform> style: TrailStyle;

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

fn create_vert(p: TrailPoint, side: bool) -> VertexOut {
    let right = cross(normalize(p.velocity), vec3f(0.0, 0.0, -1.0));
    let width = p.width;
    return VertexOut(
        vec4f(p.position + select(-width, width, side) * right, 1.0),
        p.color
    );
}

@vertex
fn vertex(@builtin(vertex_index) vidx: u32) -> VertexOut {
    let side = vidx % 2u == 0u;
    let idx = vidx / 2u;
    let point = data[(header.head - idx - 1 + header.capacity) % header.capacity];
    return create_vert(point, side);
}

@fragment
fn fragment(in: VertexOut) -> @location(0) vec4<f32> {
    return in.color;
}
