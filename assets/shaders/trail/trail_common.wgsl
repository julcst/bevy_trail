#define_import_path trail::common

struct TrailUniforms {
    ring_state: vec4<u32>,
    style: vec4<f32>,
    custom_a: vec4<f32>,
    custom_b: vec4<f32>,
};

struct TrailPointData {
    position: vec3<f32>,
    width: f32,
    color: vec4<f32>,
    velocity: vec3<f32>,
    length: f32,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> trail: TrailUniforms;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<storage, read> trail_points: array<vec4<f32>>;

fn trail_ring_start() -> u32 {
    let head = trail.ring_state.x;
    let len = trail.ring_state.y;
    let capacity = trail.ring_state.z;
    return (head + capacity - len) % capacity;
}

fn trail_logical_to_physical(logical_index: u32) -> u32 {
    let capacity = trail.ring_state.z;
    return (trail_ring_start() + logical_index) % capacity;
}

fn trail_read_point(physical_index: u32) -> TrailPointData {
    let base = physical_index * 3u;
    let p0 = trail_points[base];
    let p1 = trail_points[base + 1u];
    let p2 = trail_points[base + 2u];
    return TrailPointData(p0.xyz, p0.w, p1, p2.xyz, p2.w);
}

fn trail_safe_tangent(logical_index: u32) -> vec3<f32> {
    let len = trail.ring_state.y;
    let prev_idx = select(logical_index - 1u, 0u, logical_index == 0u);
    let next_idx = min(logical_index + 1u, len - 1u);

    let prev_point = trail_read_point(trail_logical_to_physical(prev_idx));
    let next_point = trail_read_point(trail_logical_to_physical(next_idx));

    let tangent = next_point.position - prev_point.position;
    if dot(tangent, tangent) < 0.000001 {
        return vec3<f32>(1.0, 0.0, 0.0);
    }
    return normalize(tangent);
}
