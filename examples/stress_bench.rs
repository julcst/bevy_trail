//! Stress benchmark: spawn many moving trails and report frame-time stats.
//!
//! Run with the number of trails as the first arg (default 100):
//!     cargo run --release --example stress_bench -- 100
//!
//! It measures the per-frame `Update`+render CPU time via the frame-time
//! diagnostic, skips a warmup period, then prints mean / p50 / p95 / max frame
//! times and exits, so it can be driven repeatedly from a script.

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::PresentMode,
};
use bevy_trail::prelude::*;
use rand::Rng;

#[derive(Component)]
struct Orbiter {
    center: Vec3,
    radius: f32,
    speed: f32,
    phase: f32,
    axis: f32,
}

#[derive(Resource)]
struct Bench {
    warmup: f32,
    duration: f32,
    samples: Vec<f32>,
    elapsed: f32,
}

fn main() {
    let count: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    // Disable vsync so we measure actual CPU/GPU throughput, not
                    // the display refresh rate.
                    present_mode: PresentMode::AutoNoVsync,
                    title: format!("bevy_trail stress: {count} trails"),
                    ..default()
                }),
                ..default()
            }),
        )
        .add_plugins((FrameTimeDiagnosticsPlugin::default(), TrailPlugin))
        .insert_resource(TrailCount(count))
        .insert_resource(Bench {
            warmup: 2.0,
            duration: 6.0,
            samples: Vec::new(),
            elapsed: 0.0,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (orbit_system, record_system))
        .run();
}

#[derive(Resource)]
struct TrailCount(usize);

fn setup(mut commands: Commands, count: Res<TrailCount>) {
    let mut rng = rand::rng();
    let n = count.0;

    for _ in 0..n {
        let center = Vec3::new(
            rng.random_range(-6.0..6.0),
            rng.random_range(-3.5..3.5),
            rng.random_range(-3.0..3.0),
        );
        commands.spawn((
            Transform::from_translation(center),
            Trail::new(128).with_max_length(4.0).with_max_time(2.0),
            TrailStyle {
                start_color: LinearRgba::new(
                    rng.random_range(0.3..1.0),
                    rng.random_range(0.3..1.0),
                    rng.random_range(0.3..1.0),
                    1.0,
                ),
                end_color: LinearRgba::new(0.0, 0.0, 0.0, 0.0),
                start_width: 0.25,
                end_width: 0.0,
                ..default()
            },
            TrailEmitter::default(),
            Orbiter {
                center,
                radius: rng.random_range(0.6..1.4),
                speed: rng.random_range(1.5..3.5),
                phase: rng.random_range(0.0..std::f32::consts::TAU),
                axis: rng.random_range(0.0..1.0),
            },
        ));
    }

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    info!("Spawned {n} trails");
}

fn orbit_system(time: Res<Time>, mut movers: Query<(&mut Transform, &Orbiter)>) {
    let t = time.elapsed_secs();
    for (mut transform, o) in &mut movers {
        let a = t * o.speed + o.phase;
        let x = o.radius * a.cos();
        let y = o.radius * a.sin();
        transform.translation = o.center
            + Vec3::new(x, y * (1.0 - o.axis), y * o.axis)
            + Vec3::new(0.0, 0.0, o.radius * (a * 0.5).sin() * o.axis);
    }
}

fn record_system(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    mut bench: ResMut<Bench>,
    mut exit: MessageWriter<AppExit>,
) {
    bench.elapsed += time.delta_secs();
    if bench.elapsed < bench.warmup {
        return;
    }

    if let Some(frame_time) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.value())
    {
        bench.samples.push(frame_time as f32);
    }

    if bench.elapsed >= bench.warmup + bench.duration {
        let mut s = bench.samples.clone();
        s.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = s.len().max(1);
        let mean = s.iter().sum::<f32>() / n as f32;
        let pct = |p: f32| s[((p * (n as f32 - 1.0)) as usize).min(n - 1)];
        info!(
            "RESULT frames={} mean={:.3}ms p50={:.3}ms p95={:.3}ms p99={:.3}ms max={:.3}ms (~{:.0} fps mean)",
            n,
            mean,
            pct(0.50),
            pct(0.95),
            pct(0.99),
            s[n - 1],
            1000.0 / mean,
        );
        exit.write(AppExit::Success);
    }
}
