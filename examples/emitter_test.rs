use bevy::{
    camera::{primitives::Aabb, visibility::NoFrustumCulling},
    prelude::*,
    render::storage::ShaderStorageBuffer,
};
use bevy_trail::{
    emitter::{TrailEmitter, TrailEmitterPlugin},
    render::TrailRenderPlugin,
    types::{TrailData, TrailHeader, TrailPoint, TrailStyle},
};

#[derive(Component)]
struct CircleMover {
    radius: f32,
    speed: f32,
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TrailRenderPlugin, TrailEmitterPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, move_in_circle_system)
        .run();
}

fn setup(mut commands: Commands, mut buffers: ResMut<Assets<ShaderStorageBuffer>>) {
    let capacity = 256;
    let cpu_data = vec![TrailPoint::default(); capacity as usize];
    let data = buffers.add(ShaderStorageBuffer::from(cpu_data.clone()));

    commands.spawn((
        Visibility::Visible,
        NoFrustumCulling,
        Transform::from_xyz(0.8, 0.0, 0.0),
        Aabb {
            center: Vec3A::ZERO,
            half_extents: Vec3A::splat(1.5),
        },
        TrailEmitter::default(),
        TrailData {
            header: TrailHeader {
                head: 0,
                length: 0,
                capacity,
            },
            data,
            cpu_data,
            style: TrailStyle {
                taper: 0.4,
                fade: 0.0,
                profile: 0,
            },
        },
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
