use bevy::prelude::*;
use bevy_trail::prelude::{
    TrailEmitter, TrailEmitterConfig, TrailMetadata, TrailPlugin, TrailSamplingConfig,
};

#[derive(Component)]
struct TrailDemoMover {
    radius: f32,
    angular_speed: f32,
    bob_height: f32,
    bob_speed: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TrailPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, animate_mover)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-6.0, 7.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 40_000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, -0.5, 0.0)),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(30.0, 30.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.03, 0.04, 0.06),
            perceptual_roughness: 1.0,
            ..default()
        })),
    ));

    let trail_config = TrailEmitterConfig::new()
        .with_sampling(TrailSamplingConfig {
            min_sample_dt: 1.0 / 240.0,
            distance_threshold: 0.02,
            max_sample_dt: 1.0 / 90.0,
            max_points: 2048,
            point_lifetime_secs: 4.0,
        })
        .with_metadata(TrailMetadata {
            base_width: 0.4,
            taper_factor: 0.99,
            custom_0: Vec4::new(0.25, 0.85, 1.0, 1.0),
            custom_1: Vec4::ZERO,
        })
        .with_color_fn(|pos, vel| {
            let speed = vel.length().clamp(0.0, 10.0) / 10.0;
            let heat = (pos.y * 0.5 + 0.5).clamp(0.0, 1.0);
            Vec4::new(
                0.15 + speed * 0.55,
                0.45 + heat * 0.45,
                0.95,
                0.75 + speed * 0.25,
            )
        });

    commands.spawn((
        TrailDemoMover {
            radius: 3.0,
            angular_speed: 1.7,
            bob_height: 0.8,
            bob_speed: 2.8,
        },
        Mesh3d(meshes.add(Sphere::new(0.22))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.9, 1.0),
            emissive: Color::srgb(0.05, 0.3, 0.4).into(),
            ..default()
        })),
        Transform::from_xyz(3.0, 1.2, 0.0),
        TrailEmitter,
        trail_config,
    ));
}

fn animate_mover(time: Res<Time>, mut movers: Query<(&TrailDemoMover, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (mover, mut transform) in &mut movers {
        let angle = t * mover.angular_speed;
        transform.translation = Vec3::new(
            angle.cos() * mover.radius,
            1.2 + (t * mover.bob_speed).sin() * mover.bob_height,
            angle.sin() * mover.radius,
        );
    }
}
