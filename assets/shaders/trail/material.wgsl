#import bevy_pbr::mesh_view_bindings::view

#import trail::common::{
    trail,
    trail_read_point,
    trail_logical_to_physical,
    trail_safe_tangent,
    TrailPointData,
}

struct TrailStyle {
    taper: f32,
    fade: f32,
    profile: u32,
};

fn get_edge(style: TrailStyle, side: f32) -> f32 {
    switch style.profile {
        case 0u, default { // flat
            return 1.0;
        } case 1u { // round
            return smoothstep(1.0, 0.0, abs(side));
        } case 2u { // triangle
            return 1.0 - abs(side);
        }
    }
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> trail: Trail;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<storage, read> trail_points: array<TrailPoint>;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> style: TrailStyle;

struct VertexIn {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) _position: vec3<f32>,
};

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) side: f32,
    @location(2) age: f32,
    @location(3) trail_length: f32,
};

@vertex
fn vertex(in: VertexIn) -> VertexOut {
    let logical_index = (in.vertex_index / 2u) % trail.capacity;
    let physical_index = trail_logical_to_physical(logical_index);
    let trail_point = trail_points[trail_index];
    let right = trail_right(trail_point);
    let side = select(-right, right, in.vertex_index % 2u == 1u);

    let age = f32(logical_index) / max(1.0, f32(trail.length - 1u));
    let taper = mix(1.0, 1.0 - style.taper, age);
    let half_width = point.width * 0.5 * taper;
    let world_pos = point.position + right * side * half_width;

    out.clip_pos = view.clip_from_world * vec4<f32>(world_pos, 1.0);
    out.color = mix(point.color, point.color * (1.0 - style.fade), age);
    out.side = side;
    out.age = age;
    out.trail_length = point.length;
}

@fragment
fn fragment(in: VertexOut) -> @location(0) vec4<f32> {
    let edge = get_edge(style, in.side);
    let rgb = vec3<f32>(1.0, 0.0, 0.0) * edge;
    return vec4<f32>(rgb, 0.0);
}
