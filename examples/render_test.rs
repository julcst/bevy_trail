//! Low-level example: render a pre-baked, static trail.
//!
//! Most users should reach for [`Trail`] + [`TrailEmitter`] (see
//! `emitter_test`). This example instead inserts the internal [`TrailData`]
//! directly to show the escape hatch for feeding custom, pre-computed geometry.
//! The points are world-space and drawn by the global batched pass, so no
//! `Transform`, `Visibility`, or `Aabb` is needed.

use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy_trail::{
    types::{TrailData, TrailPoint, TrailStyle},
    TrailPlugin,
};

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, TrailPlugin))
        .add_systems(Startup, setup);
    app.run();
}

/// Spawns the objects in the scene.
fn setup(mut commands: Commands) {
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

    // `from_points` builds the ring header for us; pairing it with a `TrailStyle`
    // is enough — the renderer batches the world-space points into the shared GPU
    // buffers. `TrailRenderMode` is supplied via `#[require]`.
    commands.spawn((
        TrailData::from_points(cpu_data, 1.0, 1.0),
        TrailStyle {
            start_color: LinearRgba::WHITE,
            end_color: LinearRgba::RED,
            start_width: 0.05,
            ..default()
        },
    ));

    // Spawn the camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
