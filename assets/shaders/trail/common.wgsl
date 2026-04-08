#import bevy_pbr::mesh_view_bindings::view

struct Trail {
    head: u32,
    length: u32,
    capacity: u32,
};

fn trail_logical_to_physical(self: Trail, logical_index: u32) -> u32 {
    return (self.head + self.capacity - self.logical_index - 1u) % self.capacity;
}

fn trail_start(self: Trail) -> u32 {
    return (self.head + self.capacity - self.length - 1u) % self.capacity;
}

fn trail_next(self: T Trail, il, index: u32) -> u32 {
    return (index + 1u) % self.capacity;
}

struct TrailPoint {
    position: vec3f,
    width: f32,
    color: vec4f,
    velocity: vec3f,
    length: f32,
};

fn trail_right(point: TrailPoint) -> vec3f {
    let to_camera = normalize(view.world_position - point.position);
    var dir = normalize(point.velocity);
    var right = cross(dir, to_camera);
    return right;
}
