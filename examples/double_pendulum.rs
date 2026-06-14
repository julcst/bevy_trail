//! A field of trails, each driven by two independently rotating "arms".
//!
//! Summing two rotating vectors (each its own random axis, radius, speed and
//! phase) traces a smooth, chaotic-looking 3D curve that stays inside a sphere
//! of radius `arm1.radius + arm2.radius`. With random parameters per trail the
//! whole field fills a sphere with desynchronized, colorful paths.
//!
//! On-screen buttons switch the [`TrailRenderMode`] (normal / additive /
//! transparent) and the [`TrailProfile`] (flat / smooth / triangle) for every
//! trail at once, so you can see how blending and cross-section shape interact.

use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy_trail::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};

const TRAIL_COUNT: usize = 42;
const TRAIL_CAPACITY: u32 = 512;
const CAMERA_RADIUS: f32 = 6.0;

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.17);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.28);
const SELECTED_BUTTON: Color = Color::srgb(0.2, 0.5, 0.9);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TrailPlugin))
        .insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.04)))
        .init_resource::<Controls>()
        .add_systems(Startup, (setup, setup_ui))
        .add_systems(
            Update,
            (
                move_trails,
                spin_camera,
                (handle_buttons, apply_controls, highlight_buttons).chain(),
            ),
        )
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
    fn random(rng: &mut impl Rng) -> Self {
        Self {
            axis: random_unit_vector(rng),
            radius: rng.random_range(0.3f32..1.0).powf(1.0 / 3.0),
            speed: rng.random_range(0.5..2.5),
            phase: rng.random_range(0.0..TAU),
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
    fn random(rng: &mut impl Rng) -> Self {
        Self {
            arm1: Arm::random(rng),
            arm2: Arm::random(rng),
        }
    }

    fn position(&self, t: f32) -> Vec3 {
        self.arm1.offset(t) + self.arm2.offset(t)
    }
}

/// A uniformly distributed point on the unit sphere.
fn random_unit_vector(rng: &mut impl Rng) -> Vec3 {
    let z = rng.random_range(-1.0f32..1.0);
    let angle = rng.random_range(0.0f32..TAU);
    let r = (1.0 - z * z).sqrt();
    Vec3::new(r * angle.cos(), r * angle.sin(), z)
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ball = meshes.add(Sphere::new(0.04).mesh().uv(12, 6));

    for i in 0..TRAIL_COUNT {
        let mut rng = StdRng::seed_from_u64(i as u64);

        let pendulum = DoublePendulum::random(&mut rng);
        let color = Oklcha::lch(
            rng.random_range(0.6..1.0),
            1.0,
            rng.random_range(0.0..360.0),
        );

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

// --- UI -------------------------------------------------------------------

/// The render mode and profile currently applied to every trail.
#[derive(Resource)]
struct Controls {
    mode: TrailRenderMode,
    profile: TrailProfile,
}

impl Default for Controls {
    fn default() -> Self {
        // Additive + smooth shows off both alpha blending and the rounded
        // cross-section out of the box.
        Self {
            mode: TrailRenderMode::Additive,
            profile: TrailProfile::Smooth,
        }
    }
}

/// Marks a button with the setting it selects when clicked.
#[derive(Component, Clone, Copy)]
enum Control {
    Mode(TrailRenderMode),
    Profile(TrailProfile),
}

fn setup_ui(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|root| {
            spawn_row(
                root,
                "Render mode",
                &[
                    ("Normal", Control::Mode(TrailRenderMode::Normal)),
                    ("Additive", Control::Mode(TrailRenderMode::Additive)),
                    ("Transparent", Control::Mode(TrailRenderMode::Transparent)),
                ],
            );
            spawn_row(
                root,
                "Profile",
                &[
                    ("Flat", Control::Profile(TrailProfile::Flat)),
                    ("Smooth", Control::Profile(TrailProfile::Smooth)),
                    ("Triangle", Control::Profile(TrailProfile::Triangle)),
                ],
            );
        });
}

/// Spawns a labeled row of buttons, one per `(label, control)` pair.
fn spawn_row(parent: &mut ChildSpawnerCommands, title: &str, buttons: &[(&str, Control)]) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(title),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.75)),
            ));
            row.spawn(Node {
                column_gap: Val::Px(6.0),
                ..default()
            })
            .with_children(|buttons_row| {
                for (label, control) in buttons {
                    buttons_row
                        .spawn((
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                                ..default()
                            },
                            BackgroundColor(NORMAL_BUTTON),
                            *control,
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new(*label),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                }
            });
        });
}

/// Records the chosen render mode / profile when a button is pressed.
fn handle_buttons(
    interactions: Query<(&Interaction, &Control), (Changed<Interaction>, With<Button>)>,
    mut controls: ResMut<Controls>,
) {
    for (interaction, control) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match control {
            Control::Mode(mode) => controls.mode = *mode,
            Control::Profile(profile) => controls.profile = *profile,
        }
    }
}

/// Pushes the current selection onto every trail when it changes.
fn apply_controls(
    controls: Res<Controls>,
    mut modes: Query<&mut TrailRenderMode>,
    mut styles: Query<&mut TrailStyle>,
) {
    if !controls.is_changed() {
        return;
    }
    for mut mode in &mut modes {
        *mode = controls.mode;
    }
    for mut style in &mut styles {
        style.profile = controls.profile as u32;
    }
}

/// Tints each button: highlighted when selected, lit when hovered.
fn highlight_buttons(
    controls: Res<Controls>,
    mut buttons: Query<(&Control, &Interaction, &mut BackgroundColor)>,
) {
    for (control, interaction, mut background) in &mut buttons {
        let selected = match control {
            Control::Mode(mode) => *mode == controls.mode,
            Control::Profile(profile) => *profile == controls.profile,
        };
        background.0 = if selected {
            SELECTED_BUTTON
        } else if *interaction == Interaction::Hovered {
            HOVERED_BUTTON
        } else {
            NORMAL_BUTTON
        };
    }
}
