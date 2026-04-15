use std::f32::consts::PI;

use bevy::{
    camera::{primitives::Aabb, visibility::NoFrustumCulling},
    prelude::*,
    render::storage::ShaderStorageBuffer,
};
use bevy_trail::{
    render::TrailRenderPlugin,
    types::{TrailData, TrailHeader, TrailPoint, TrailStyle},
};

const CAPACITY: u32 = 128;
const RADIUS: f32 = 0.45;
const ANGULAR_SPEED: f32 = 1.8;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, TrailRenderPlugin))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, animate_trail);
    app.run();
}

/// Spawns the objects in the scene.
fn setup(mut commands: Commands, mut buffers: ResMut<Assets<ShaderStorageBuffer>>) {
    let n = CAPACITY;
    let initial = TrailPoint {
        position: Vec3::new(0.5, 0.0, 0.0),
        width: 0.05,
        color: Vec4::new(0.2, 0.9, 0.8, 1.0),
        velocity: Vec3::Y,
        t: 0.0,
    };
    let cpu_data = vec![initial; n as usize];

    let header = TrailHeader {
        head: 0,
        length: 0,
        capacity: n,
    };

    let style = TrailStyle {
        taper: 0.5,
        fade: 0.5,
        profile: 0,
    };

    let data = buffers.add(ShaderStorageBuffer::from(cpu_data.clone()));

    // Spawn a single entity that has custom rendering. It'll be extracted into
    // the render world via [`ExtractComponent`].
    commands.spawn((
        Aabb {
            center: Vec3A::ZERO,
            half_extents: Vec3A::splat(0.5),
        },
        NoFrustumCulling,
        Visibility::Visible,
        Transform::default(),
        GlobalTransform::default(),
        TrailData {
            header,
            data,
            cpu_data,
            style,
        },
    ));

    // Spawn the camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Appends one point per frame and advances the ring-buffer head.
fn animate_trail(
    time: Res<Time>,
    mut trails: Query<&mut TrailData>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    let t = time.elapsed_secs();
    let theta = (t * ANGULAR_SPEED).rem_euclid(2.0 * PI);

    let position = Vec3::new(theta.cos() * RADIUS, theta.sin() * RADIUS, 0.0);

    for mut trail in &mut trails {
        let cap = trail.header.capacity as usize;
        trail.cpu_data.resize_with(cap, Default::default);

        let index = trail.header.head as usize;
        trail.cpu_data[index] = TrailPoint {
            position,
            width: 0.05,
            color: Vec4::new(0.2, 0.9, 0.8, 1.0),
            t,
        };

        trail.header.head = (trail.header.head + 1) % trail.header.capacity;
        trail.header.length = (trail.header.length + 1).min(trail.header.capacity);

        if let Some(buffer) = buffers.get_mut(&trail.data) {
            buffer.set_data(trail.cpu_data.clone());
        }
    }
}
