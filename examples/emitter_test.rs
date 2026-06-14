use bevy::prelude::*;
use bevy_trail::prelude::*;

#[derive(Component)]
struct CircleMover {
    radius: f32,
    speed: f32,
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TrailPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, move_in_circle_system)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Transform::from_xyz(0.8, 0.0, 0.0),
        Trail::new(64).with_max_length(2.0).with_max_time(2.0),
        TrailStyle {
            start_color: LinearRgba::WHITE,
            end_color: LinearRgba::RED,
            start_width: 0.05,
            ..default()
        },
        TrailEmitter::default(),
        CircleMover {
            radius: 0.8,
            speed: 1.5,
        },
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 2.5).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn move_in_circle_system(time: Res<Time>, mut movers: Query<(&mut Transform, &CircleMover)>) {
    for (mut transform, mover) in &mut movers {
        let angle = time.elapsed_secs() * mover.speed;
        transform.translation =
            Vec3::new(mover.radius * angle.cos(), mover.radius * angle.sin(), 0.0);
    }
}
