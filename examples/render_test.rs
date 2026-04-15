use std::f32::consts::TAU;

use bevy::{
    camera::{primitives::Aabb, visibility::NoFrustumCulling},
    prelude::*,
    render::storage::ShaderStorageBuffer,
};
use bevy_trail::{
    render::TrailRenderPlugin,
    types::{TrailData, TrailHeader, TrailPoint, TrailStyle},
};

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, TrailRenderPlugin))
        .add_systems(Startup, setup);
    app.run();
}

/// Spawns the objects in the scene.
fn setup(mut commands: Commands, mut buffers: ResMut<Assets<ShaderStorageBuffer>>) {
    // Generate sine wave
    let n = 128;
    let cpu_data = (0..n)
        .map(|i| {
            let t = i as f32 / (n as f32 - 1.0);
            TrailPoint {
                position: Vec3::new(t - 0.5, (t * TAU).sin() * 0.5, 0.0),
                width: 0.05,
                color: Vec4::new(t, 1.0 - t, 0.5, 1.0),
                t,
            }
        })
        .collect::<Vec<_>>();

    let header = TrailHeader {
        head: 0,
        length: n,
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
        // Note: Aabb would be better
        // Aabb {
        //     center: Vec3A::ZERO,
        //     half_extents: Vec3A::splat(0.5),
        // },
        Visibility::Visible,
        Transform::default(),
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
