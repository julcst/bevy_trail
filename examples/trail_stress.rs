use bevy::prelude::*;
use bevy_trail::prelude::{
    TrailEmitter, TrailEmitterConfig, TrailMetadata, TrailPlugin, TrailSamplingConfig,
};

const GRID_X: i32 = 20;
const GRID_Z: i32 = 20;
const SPACING: f32 = 1.5;

#[derive(Component)]
struct StressMover {
    base: Vec3,
    phase: f32,
    speed: f32,
    amplitude: Vec3,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TrailPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, animate_stress_movers)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 20.0, 36.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            shadows_enabled: false,
            illuminance: 30_000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, -0.35, 0.0)),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(80.0, 80.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.03, 0.035, 0.05),
            perceptual_roughness: 1.0,
            ..default()
        })),
    ));

    let trail_config = TrailEmitterConfig::new()
        .with_sampling(TrailSamplingConfig {
            min_sample_dt: 1.0 / 240.0,
            distance_threshold: 0.02,
            max_sample_dt: 1.0 / 90.0,
            max_points: 1024,
            point_lifetime_secs: 2.5,
        })
        .with_metadata(TrailMetadata {
            base_width: 0.08,
            taper_factor: 0.9,
            custom_0: Vec4::new(0.4, 0.9, 1.0, 1.0),
            custom_1: Vec4::ZERO,
        })
        .with_color_fn(|pos, vel| {
            let speed = (vel.length() / 8.0).clamp(0.0, 1.0);
            let heat = (pos.y * 0.5 + 0.5).clamp(0.0, 1.0);
            Vec4::new(
                0.2 + 0.5 * speed,
                0.4 + 0.5 * heat,
                0.95,
                0.35 + 0.65 * speed,
            )
        });

    let marker_mesh = meshes.add(Sphere::new(0.08));
    let marker_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.65, 0.95, 1.0),
        emissive: Color::srgb(0.2, 0.7, 0.95).into(),
        ..default()
    });

    for ix in 0..GRID_X {
        for iz in 0..GRID_Z {
            let x = (ix as f32 - GRID_X as f32 * 0.5) * SPACING;
            let z = (iz as f32 - GRID_Z as f32 * 0.5) * SPACING;
            let idx = (ix * GRID_Z + iz) as f32;

            commands.spawn((
                StressMover {
                    base: Vec3::new(x, 1.2, z),
                    phase: idx * 0.17,
                    speed: 1.4 + (idx * 0.013).sin().abs() * 2.2,
                    amplitude: Vec3::new(0.45, 0.55, 0.45),
                },
                Mesh3d(marker_mesh.clone()),
                MeshMaterial3d(marker_material.clone()),
                Transform::from_xyz(x, 1.2, z),
                TrailEmitter,
                trail_config.clone(),
            ));
        }
    }
}

fn animate_stress_movers(time: Res<Time>, mut movers: Query<(&StressMover, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (mover, mut transform) in &mut movers {
        let p = t * mover.speed + mover.phase;
        transform.translation = mover.base
            + Vec3::new(
                p.cos() * mover.amplitude.x,
                (p * 1.7).sin() * mover.amplitude.y,
                (p * 1.3).sin() * mover.amplitude.z,
            );
    }
}
