//! Renders a handful of moving trails and saves a screenshot to
//! `/tmp/trail_shot.png`, then exits. Used to visually verify the batched
//! renderer actually draws correct geometry (not just that it runs).

use bevy::{
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot},
};
use bevy_trail::prelude::*;

#[derive(Component)]
struct Mover {
    base: Vec3,
    phase: f32,
}

#[derive(Resource)]
struct FrameCount(u32);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TrailPlugin))
        .insert_resource(FrameCount(0))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_trails, capture))
        .run();
}

fn setup(mut commands: Commands) {
    // Cycle blend modes across the trails so the batched renderer's per-mode
    // grouping (one instanced draw per mode) is exercised.
    let modes = [
        TrailRenderMode::Opaque,
        TrailRenderMode::Additive,
        TrailRenderMode::Transparent,
    ];
    for i in 0..6 {
        let base = Vec3::new(-5.0 + i as f32 * 2.0, 0.0, 0.0);
        commands.spawn((
            Transform::from_translation(base),
            Trail::new(96).with_max_length(20.0).with_max_time(20.0),
            TrailStyle {
                start_color: LinearRgba::new(1.0, 0.35, 0.1, 1.0),
                end_color: LinearRgba::new(0.1, 0.4, 1.0, 1.0),
                start_width: 0.35,
                end_width: 0.04,
                ..default()
            },
            modes[i % 3],
            TrailEmitter::default(),
            Mover {
                base,
                phase: i as f32 * 0.7,
            },
        ));
    }

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn move_trails(time: Res<Time>, mut movers: Query<(&mut Transform, &Mover)>) {
    let t = time.elapsed_secs();
    for (mut transform, m) in &mut movers {
        let a = t * 2.5 + m.phase;
        transform.translation = m.base + Vec3::new(a.cos() * 1.4, a.sin() * 1.4, 0.0);
    }
}

fn capture(
    mut frame: ResMut<FrameCount>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    frame.0 += 1;
    // Give the trails time to lay down points before grabbing the frame.
    if frame.0 == 150 {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk("/tmp/trail_shot.png"));
        info!("screenshot requested");
    }
    if frame.0 == 170 {
        exit.write(AppExit::Success);
    }
}
