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

fn get_point(idx: u32) -> TrailPoint {
    return data[(header.head + header.capacity - idx) % header.capacity];
}

fn calc_curvature(idx: u32) -> vec3f {
    let prev = get_point(idx - 1u).position;
    let curr = get_point(idx).position;
    let next = get_point(idx + 1u).position;
    return normalize(prev - 2.0 * curr + next);
}

fn calc_tangent(idx: u32) -> vec3f {
    var t: vec3f;
    if idx == 0u {
        t = get_point(idx + 1u).position - get_point(idx).position;
    } else if idx == header.length - 1u {
        t = get_point(idx).position - get_point(idx - 1u).position;
    } else {
        t = get_point(idx + 1u).position - get_point(idx - 1u).position;
    }
    return normalize(t);
}

@vertex
fn vertex(@builtin(vertex_index) vidx: u32) -> VertexOut {
    let side = vidx % 2u == 0u;
    let idx = vidx / 2u;
    let curr = get_point(idx);
    let forward = calc_tangent(idx);
    let right = cross(forward, vec3f(0.0, 0.0, 1.0)) * select(-curr.width, curr.width, side);
    return VertexOut(
        vec4f(curr.position + right, 1.0),
        curr.color
    );
}

@fragment
fn fragment(in: VertexOut) -> @location(0) vec4<f32> {
    return in.color;
}
