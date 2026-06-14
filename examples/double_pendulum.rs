//! A field of trails, each driven by two independently rotating "arms".
//!
//! Summing two rotating vectors (each its own random axis, radius, speed and
//! phase) traces a smooth, chaotic-looking 3D curve that stays inside a sphere
//! of radius `arm1.radius + arm2.radius`. With random parameters per trail the
//! whole field fills a sphere with desynchronized, colorful paths.

use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy_trail::prelude::*;

const TRAIL_COUNT: usize = 42;
const TRAIL_CAPACITY: u32 = 512;
const CAMERA_RADIUS: f32 = 6.0;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TrailPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_trails, spin_camera))
        .run();
}

/// One rotating arm: a vector of length `radius` spun around `axis`.
struct Arm {
    axis: Vec3,
    radius: f32,
    speed: f32,
    phase: f32,
}

impl Arm {
    fn random(rng: &mut Rng) -> Self {
        Self {
            axis: rng.unit_vector(),
            radius: rng.range(0.3, 1.0).powf(1.0 / 3.0),
            speed: rng.range(0.5, 2.5),
            phase: rng.range(0.0, TAU),
        }
    }

    fn offset(&self, t: f32) -> Vec3 {
        Quat::from_axis_angle(self.axis, self.phase + self.speed * t) * (Vec3::X * self.radius)
    }
}

/// Two arms whose summed offset traces the trail's path.
#[derive(Component)]
struct DoublePendulum {
    arm1: Arm,
    arm2: Arm,
}

impl DoublePendulum {
    fn random(rng: &mut Rng) -> Self {
        Self {
            arm1: Arm::random(rng),
            arm2: Arm::random(rng),
        }
    }

    fn position(&self, t: f32) -> Vec3 {
        self.arm1.offset(t) + self.arm2.offset(t)
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ball = meshes.add(Sphere::new(0.04).mesh().uv(12, 6));

    for i in 0..TRAIL_COUNT {
        let mut rng = Rng::seed(i as u32);

        let pendulum = DoublePendulum::random(&mut rng);
        let color = Oklcha::lch(rng.range(0.6, 1.0), 1.0, rng.range(0.0, 360.0));

        commands.spawn((
            Transform::from_translation(pendulum.position(0.0)),
            Trail::new(TRAIL_CAPACITY)
                .with_max_length(6.0)
                .with_max_time(6.0),
            TrailStyle {
                start_color: color.into(),
                end_color: color.with_alpha(0.0).into(),
                start_width: 0.015,
                end_width: 0.0,
                ..default()
            },
            TrailEmitter::default(),
            pendulum,
            // A small unlit ball marks the head of the trail.
            children![(
                Mesh3d(ball.clone()),
                MeshMaterial3d(materials.add(StandardMaterial {
                    emissive: color.into(),
                    ..default()
                })),
            )],
        ));
    }

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, CAMERA_RADIUS).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn move_trails(time: Res<Time>, mut trails: Query<(&DoublePendulum, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (pendulum, mut transform) in &mut trails {
        transform.translation = pendulum.position(t);
    }
}

fn spin_camera(time: Res<Time>, mut cameras: Query<&mut Transform, With<Camera3d>>) {
    let angle = time.elapsed_secs() * 0.2;
    for mut transform in &mut cameras {
        transform.translation =
            CAMERA_RADIUS * Vec3::new(angle.sin(), 0.2 * angle.cos(), angle.cos()).normalize();
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

/// Tiny deterministic xorshift PRNG so each trail gets its own random
/// parameters without pulling in an external crate.
struct Rng(u32);

impl Rng {
    fn seed(index: u32) -> Self {
        // Scramble the index so consecutive seeds produce unrelated streams.
        let state = index.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
        Self(state | 1)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// A uniform float in `[0, 1)`.
    fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.unit()
    }

    /// A uniformly distributed point on the unit sphere.
    fn unit_vector(&mut self) -> Vec3 {
        let z = self.range(-1.0, 1.0);
        let angle = self.range(0.0, TAU);
        let r = (1.0 - z * z).sqrt();
        Vec3::new(r * angle.cos(), r * angle.sin(), z)
    }
}
