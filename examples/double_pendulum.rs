//! A field of trails, each driven by two independently rotating "arms".
//!
//! Summing two rotating vectors (each its own random axis, radius, speed and
//! phase) traces a smooth, chaotic-looking 3D curve that stays inside a sphere
//! of radius `arm1.radius + arm2.radius`. With random parameters per trail the
//! whole field fills a sphere with desynchronized, colorful paths.
//!
//! A Bevy Feathers panel of radio buttons switches the [`TrailRenderMode`]
//! (normal / additive / transparent) and the [`TrailProfile`] (flat / smooth /
//! triangle) for every trail at once, so you can see how blending and
//! cross-section shape interact.

use core::mem;
use std::f32::consts::TAU;

use bevy::{
    feathers::{
        controls::radio,
        dark_theme::create_dark_theme,
        theme::{ThemeBackgroundColor, ThemedText, UiTheme},
        tokens, FeathersPlugins,
    },
    input_focus::tab_navigation::TabGroup,
    prelude::*,
    ui::Checked,
    ui_widgets::{observe, RadioGroup, ValueChange},
};
use bevy_trail::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};

const TRAIL_COUNT: usize = 300;
const TRAIL_CAPACITY: u32 = 512;
const CAMERA_RADIUS: f32 = 6.0;

// Transparent + smooth shows off alpha blending and the rounded cross-section
// out of the box; the matching radio buttons start checked.
const INITIAL_MODE: TrailRenderMode = TrailRenderMode::Transparent;
const INITIAL_PROFILE: TrailProfile = TrailProfile::Smooth;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins, TrailPlugin))
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.04)))
        .add_systems(Startup, (setup, setup_ui))
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
    fn random(rng: &mut impl Rng) -> Self {
        Self {
            axis: Sphere::new(1.0).sample_boundary(rng),
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
        let mut rng = StdRng::seed_from_u64(i as u64);

        let pendulum = DoublePendulum {
            arm1: Arm::random(&mut rng),
            arm2: Arm::random(&mut rng),
        };
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
                profile: INITIAL_PROFILE as u32,
            },
            INITIAL_MODE,
            TrailEmitter::default(),
            pendulum,
            // A small unlit ball marks the head of the trail. It lives on the
            // *same* entity as the emitter — a trail is not a render object, so
            // it never conflicts with the entity's own mesh.
            Mesh3d(ball.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                emissive: color.into(),
                ..default()
            })),
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

/// The trail setting a radio button selects when clicked.
#[derive(Component, Clone, Copy)]
enum Setting {
    Mode(TrailRenderMode),
    Profile(TrailProfile),
}

fn setup_ui(mut commands: Commands) {
    use Setting::{Mode, Profile};
    use TrailProfile::*;
    use TrailRenderMode::*;

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            flex_direction: FlexDirection::Column,
            row_gap: px(12),
            padding: UiRect::all(px(12)),
            ..default()
        },
        TabGroup::default(),
        ThemeBackgroundColor(tokens::WINDOW_BG),
        children![
            (Text::new("Render mode"), ThemedText),
            (
                radio_group(),
                children![
                    setting_radio(Mode(Opaque), "Opaque"),
                    setting_radio(Mode(Additive), "Additive"),
                    (setting_radio(Mode(Transparent), "Transparent"), Checked),
                ],
            ),
            (Text::new("Profile"), ThemedText),
            (
                radio_group(),
                children![
                    setting_radio(Profile(Flat), "Flat"),
                    (setting_radio(Profile(Smooth), "Smooth"), Checked),
                    setting_radio(Profile(Triangle), "Triangle"),
                ],
            ),
        ],
    ));
}

/// A vertically-stacked radio group that applies its selection on change.
fn radio_group() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            ..default()
        },
        RadioGroup,
        observe(on_select),
    )
}

/// A radio button tagged with the [`Setting`] it selects. Wrap it in a tuple
/// with [`Checked`] to start it selected.
fn setting_radio(setting: Setting, label: &'static str) -> impl Bundle {
    radio(setting, Spawn((Text::new(label), ThemedText)))
}

/// Applies the picked [`Setting`] to every trail and moves the check mark to the
/// chosen radio within its group.
fn on_select(
    change: On<ValueChange<Entity>>,
    radios: Query<(Entity, &Setting)>,
    mut commands: Commands,
    mut modes: Query<&mut TrailRenderMode>,
    mut styles: Query<&mut TrailStyle>,
) {
    let Ok((_, &selected)) = radios.get(change.value) else {
        return;
    };

    match selected {
        Setting::Mode(mode) => modes.iter_mut().for_each(|mut m| *m = mode),
        Setting::Profile(profile) => styles
            .iter_mut()
            .for_each(|mut s| s.profile = profile as u32),
    }

    for (entity, setting) in &radios {
        if mem::discriminant(setting) == mem::discriminant(&selected) {
            let mut radio = commands.entity(entity);
            if entity == change.value {
                radio.insert(Checked);
            } else {
                radio.remove::<Checked>();
            }
        }
    }
}
