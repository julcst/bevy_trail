#import bevy_pbr::mesh_view_bindings::view

#import trail::common::{
    trail,
    trail_read_point,
    trail_logical_to_physical,
    trail_safe_tangent,
}

struct VertexIn {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) _position: vec3<f32>,
};

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) side: f32,
    @location(2) age: f32,
    @location(3) trail_length: f32,
};

fn trail_vertex(logical_index: u32, side: f32) -> VertexOut {
    var out: VertexOut;

    let point = trail_read_point(trail_logical_to_physical(logical_index));
    let to_camera = normalize(view.world_position - point.position);

    var motion_dir = point.velocity;
    if dot(motion_dir, motion_dir) < 0.000001 {
        motion_dir = trail_safe_tangent(logical_index);
    }

    let motion_dir_norm = normalize(motion_dir);
    var right = cross(motion_dir_norm, to_camera);
    if dot(right, right) < 0.000001 {
        right = cross(to_camera, vec3<f32>(0.0, 1.0, 0.0));
        if dot(right, right) < 0.000001 {
            right = cross(to_camera, vec3<f32>(1.0, 0.0, 0.0));
        }
    }
    right = normalize(right);

    let len = trail.ring_state.y;
    let base_width = trail.style.x;
    let taper_factor = trail.style.y;
    let age = f32(logical_index) / max(1.0, f32(len - 1u));
    let taper = mix(1.0 - taper_factor, 1.0, age);
    let half_width = point.width * base_width * taper;
    let world_pos = point.position + right * side * half_width;

    out.clip_position = view.clip_from_world * vec4<f32>(world_pos, 1.0);
    out.color = point.color * trail.custom_a;
    out.side = side;
    out.age = age;
    out.trail_length = point.length;
    return out;
}

@vertex
fn vertex(in: VertexIn) -> VertexOut {
    if trail.ring_state.y < 2u {
        return VertexOut(
            vec4<f32>(2.0, 2.0, 2.0, 1.0),
            vec4<f32>(0.0),
            0.0,
            1.0,
            0.0,
        );
    }

    let len = trail.ring_state.y;
    let logical_index = in.vertex_index / 2u;
    if logical_index >= len {
        return trail_vertex(len - 1u, 1.0);
    }

    let side = select(-1.0, 1.0, (in.vertex_index & 1u) == 1u);
    return trail_vertex(logical_index, side);
}

@fragment
fn fragment(in: VertexOut) -> @location(0) vec4<f32> {
    let edge = smoothstep(1.0, 0.0, abs(in.side));
    let fade = in.age;
    let alpha = in.color.a * edge * fade;
    let rgb = vec3<f32>(1.0, 0.0, 0.0) * edge * fade;
    return vec4<f32>(rgb, 0.0);
}
