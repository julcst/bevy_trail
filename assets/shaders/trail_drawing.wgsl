#import bevy_render::view::View

struct Header {
    head: u32,
    length: u32,
    capacity: u32,
    current_time: f32,
    max_time: f32,
    current_length: f32,
    max_length: f32,
    // Base index of this trail's points within the shared `points` buffer.
    offset: u32,
};

struct TrailPoint {
    position: vec3f,
    time: f32,
    custom: vec3f,
    length: f32,
};

struct TrailStyle {
    start_color: vec4f,
    end_color: vec4f,
    start_width: f32,
    end_width: f32,
    profile: u32,
};

@group(0) @binding(0) var<uniform> view: View;

// All trails in this draw are batched into shared storage buffers and selected
// per-instance: instance N renders trail N. `points` holds every trail's ring
// concatenated; each trail starts at `headers[N].offset`.
@group(1) @binding(0) var<storage, read> headers: array<Header>;
@group(1) @binding(1) var<storage, read> points: array<TrailPoint>;
@group(1) @binding(2) var<storage, read> styles: array<TrailStyle>;

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    // Signed position across the ribbon width: -1 at one edge, +1 at the other.
    @location(1) side: f32,
    // Cross-section profile of this trail; flat so it isn't interpolated.
    @location(2) @interpolate(flat) profile: u32,
};

fn get_point(h: Header, idx: u32) -> TrailPoint {
    return points[h.offset + (h.head + h.capacity - idx) % h.capacity];
}

fn calc_tangent(h: Header, idx: u32) -> vec3f {
    var t: vec3f;
    if idx == 0u {
        t = get_point(h, idx + 1u).position - get_point(h, idx).position;
    } else if idx == h.length - 1u {
        t = get_point(h, idx).position - get_point(h, idx - 1u).position;
    } else {
        t = get_point(h, idx + 1u).position - get_point(h, idx - 1u).position;
    }
    return normalize(t);
}

@vertex
fn vertex(@builtin(instance_index) inst: u32, @builtin(vertex_index) vidx: u32) -> VertexOut {
    let header = headers[inst];
    let style = styles[inst];

    let side = vidx % 2u == 0u;
    let edge = select(-1.0, 1.0, side);

    // Instanced draws emit a fixed vertex count for every trail. An empty trail
    // collapses to a single point so all of its triangles are zero-area.
    if header.length == 0u {
        return VertexOut(vec4f(0.0, 0.0, 0.0, 1.0), vec4f(0.0), edge, style.profile);
    }

    // Clamp surplus vertices onto the last point: they exactly replicate the
    // trail's final ribbon edge, so the extra triangles are degenerate and
    // invisible while the real geometry is unaffected.
    let idx = min(vidx / 2u, header.length - 1u);
    let curr = get_point(header, idx);
    let forward = calc_tangent(header, idx);
    let time = (curr.time - header.current_time) / header.max_time;
    let length = (curr.length - header.current_length) / header.max_length;
    let t = clamp(time * length, 0.0, 1.0);
    let color = mix(style.start_color, style.end_color, t);
    let width = mix(style.start_width, style.end_width, t);

    // Build a camera-facing ribbon: offset each point perpendicular to both the
    // trail tangent and the direction from the camera to the point, so the
    // ribbon always faces the camera regardless of view angle.
    let view_dir = normalize(curr.position - view.world_position);
    let right = normalize(cross(forward, view_dir)) * (edge * width);
    let world_pos = curr.position + right;

    return VertexOut(
        view.clip_from_world * vec4f(world_pos, 1.0),
        color,
        edge,
        style.profile,
    );
}

// Cross-section alpha falloff across the ribbon width, selected by the profile.
// `u` is the signed distance from the center, in [-1, 1].
fn profile_alpha(profile: u32, u: f32) -> f32 {
    switch profile {
        // Smooth: rounded falloff, like the silhouette of a tube.
        case 1u: {
            return sqrt(max(0.0, 1.0 - u * u));
        }
        // Triangle: linear falloff, peaking in the middle.
        case 2u: {
            return max(0.0, 1.0 - abs(u));
        }
        // Flat (and default): constant, hard-edged ribbon.
        default: {
            return 1.0;
        }
    }
}

@fragment
fn fragment(in: VertexOut) -> @location(0) vec4<f32> {
    var color = in.color;
    color.a *= profile_alpha(in.profile, in.side);
    return color;
}
