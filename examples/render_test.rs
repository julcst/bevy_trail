//! Low-level example: render a pre-baked, static trail.
//!
//! Most users should reach for [`Trail`] + [`TrailEmitter`] (see
//! `emitter_test`). This example instead inserts the internal [`TrailData`]
//! directly to show the escape hatch for feeding custom, pre-computed geometry.
//! Visibility and frustum culling are still handled for you — no `Aabb` or
//! `NoFrustumCulling` needed.

use std::f32::consts::TAU;

use bevy::{prelude::*, render::storage::ShaderStorageBuffer};
use bevy_trail::{
    types::{TrailData, TrailHeader, TrailPoint, TrailStyle},
    TrailPlugin,
};

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, TrailPlugin))
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
                time: t,
                length: t,
                ..default()
            }
        })
        .collect::<Vec<_>>();

    let data = buffers.add(ShaderStorageBuffer::from(cpu_data.clone()));

    // A bare `TrailData` is enough; `Transform`, `Visibility`, `Aabb` and the
    // visibility class are pulled in automatically.
    commands.spawn(TrailData {
        header: TrailHeader {
            head: n - 1,
            length: n,
            capacity: n,
            max_length: 1.0,
            max_time: 1.0,
            current_length: 1.0,
            current_time: 1.0,
        },
        data,
        cpu_data,
        style: TrailStyle {
            start_color: LinearRgba::WHITE,
            end_color: LinearRgba::RED,
            start_width: 0.05,
            ..default()
        },
    });

    // Spawn the camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
