//! Core data types for trail rendering.

use std::sync::Arc;

use bevy::{
    prelude::*,
    render::{extract_component::ExtractComponent, render_resource::ShaderType},
};

/// Cross-section shape of the ribbon, modulating alpha across its width (most
/// visible with an alpha-aware [`TrailRenderMode`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum TrailProfile {
    /// Constant, hard-edged ribbon.
    #[default]
    Flat = 0,
    /// Rounded edge falloff — soft, tube-like.
    Smooth = 1,
    /// Linear falloff, peaking in the middle.
    Triangle = 2,
}

/// How a trail's pixels blend into the frame. Inserted by [`Trail`] via
/// `#[require]`; mutate at runtime to switch compositing.
#[derive(Component, ExtractComponent, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TrailRenderMode {
    /// Overwrites the background; alpha ignored.
    #[default]
    Opaque,
    /// Adds color to the frame, scaled by alpha. Good for glow; order-independent.
    Additive,
    /// Standard alpha blending.
    Transparent,
}

/// Appearance of a trail: colors, widths, and cross-section profile.
///
/// A [`Component`] in its own right, extracted straight off the entity, so
/// mutating it animates the trail with no extra bookkeeping. [`Trail`] inserts a
/// default via `#[require]`.
#[derive(Component, ExtractComponent, Clone, Debug, ShaderType)]
pub struct TrailStyle {
    pub start_color: LinearRgba,
    pub end_color: LinearRgba,
    pub start_width: f32,
    pub end_width: f32,
    /// [`TrailProfile`] as its `u32` repr (to cross into the shader). Set via
    /// [`with_profile`](Self::with_profile).
    pub profile: u32,
}

impl Default for TrailStyle {
    fn default() -> Self {
        Self {
            start_color: LinearRgba::WHITE,
            end_color: LinearRgba::WHITE,
            start_width: 0.01,
            end_width: 0.0,
            profile: TrailProfile::Flat as u32,
        }
    }
}

impl TrailStyle {
    /// Sets the ribbon's cross-section [`TrailProfile`].
    pub fn with_profile(mut self, profile: TrailProfile) -> Self {
        self.profile = profile as u32;
        self
    }
}

/// One sample along a trail's path, stored in the GPU ring buffer.
#[derive(Clone, Debug, ShaderType, Default)]
pub struct TrailPoint {
    pub position: Vec3,
    /// Seconds since the point was emitted.
    pub time: f32,
    pub custom: Vec3,
    /// Distance along the trail to this point.
    pub length: f32,
}

/// User-facing trail configuration.
///
/// Add it to **any** entity with a [`Transform`] — even one with its own
/// [`Mesh3d`], camera, or gameplay components. Points are
/// sampled in world space and drawn by a global batched pass, so a trail never
/// interferes with the entity's own rendering. Add a
/// [`TrailEmitter`](crate::emitter::TrailEmitter) to have it follow the entity.
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_trail::prelude::*;
/// # fn setup(mut commands: Commands) {
/// commands.spawn((
///     Transform::from_xyz(1.0, 0.0, 0.0),
///     Trail::new(64).with_max_length(2.0).with_max_time(2.0),
///     TrailStyle {
///         start_color: LinearRgba::WHITE,
///         end_color: LinearRgba::RED,
///         start_width: 0.05,
///         ..default()
///     },
///     TrailEmitter::default(),
/// ));
/// # }
/// ```
#[derive(Component, Clone, Debug)]
#[require(Transform, TrailStyle, TrailRenderMode)]
pub struct Trail {
    /// Maximum number of points retained in the ring buffer.
    pub capacity: u32,
    /// Trail is clipped once it grows longer than this (world units).
    /// Set to `0.0` to disable length-based clipping.
    pub max_length: f32,
    /// Trail is clipped once its tail is older than this (seconds).
    /// Set to `0.0` to disable time-based clipping.
    pub max_time: f32,
}

impl Default for Trail {
    fn default() -> Self {
        Self {
            capacity: 128,
            max_length: 1.0,
            max_time: 1.0,
        }
    }
}

impl Trail {
    /// A trail with the given ring-buffer capacity and default clipping.
    pub fn new(capacity: u32) -> Self {
        Self {
            capacity,
            ..default()
        }
    }

    pub fn with_max_length(mut self, max_length: f32) -> Self {
        self.max_length = max_length;
        self
    }

    pub fn with_max_time(mut self, max_time: f32) -> Self {
        self.max_time = max_time;
        self
    }
}

/// Header uniform describing the live segment of the ring buffer. Maintained by
/// the plugin; not constructed by users directly.
#[derive(Clone, Debug, ShaderType)]
pub struct TrailHeader {
    /// Index of the next point to be overwritten
    pub head: u32,
    pub length: u32,
    pub capacity: u32,
    pub current_time: f32,
    /// Set to 0 to disable time-based clipping
    pub max_time: f32,
    pub current_length: f32,
    /// Set to 0 to disable length-based clipping
    pub max_length: f32,
    /// Base index of this trail's points in the batched `points` buffer. Set by
    /// the renderer per frame; `0` otherwise.
    pub offset: u32,
}

impl Default for TrailHeader {
    fn default() -> Self {
        Self {
            head: 0,
            length: 0,
            capacity: 128,
            current_time: 0.0,
            max_time: 1.0,
            current_length: 0.0,
            max_length: 1.0,
            offset: 0,
        }
    }
}

/// Renderable trail state: the world-space ring buffer plus its [`TrailHeader`].
///
/// Plain data, not a render object — no `Visibility`, `Aabb`, or visibility
/// class. The renderer collects every trail's data each frame and packs it into
/// shared buffers for one batched draw (see [`crate::render`]).
///
/// The plugin inserts and maintains this from [`Trail`]; you rarely build it
/// yourself. Use [`new`] or [`from_points`] (e.g. for a pre-baked static trail),
/// never the struct literal — they uphold the invariant below.
///
/// # Invariant
///
/// `cpu_data.len() == header.capacity` and `header.length <= header.capacity`.
/// The constructors establish it, the emitter preserves it, the renderer relies
/// on it.
///
/// [`new`]: Self::new
/// [`from_points`]: Self::from_points
#[derive(Clone, Component)]
#[require(TrailStyle, TrailRenderMode)]
pub struct TrailData {
    pub header: TrailHeader,
    /// Live ring points in **world space**, behind an [`Arc`] so extraction is a
    /// cheap pointer clone; the emitter mutates via [`Arc::make_mut`], copying
    /// only when a trail actually changes.
    pub cpu_data: Arc<Vec<TrailPoint>>,
}

impl TrailData {
    /// Empty ring storage for a trail of `capacity` points, with the given
    /// clipping budgets (`0.0` disables an axis). Used by the plugin to back a
    /// [`Trail`]; the emitter fills the ring over time.
    pub fn new(capacity: u32, max_length: f32, max_time: f32) -> Self {
        let capacity = capacity.max(1);
        Self {
            header: TrailHeader {
                capacity,
                max_length,
                max_time,
                ..default()
            },
            cpu_data: Arc::new(vec![TrailPoint::default(); capacity as usize]),
        }
    }

    /// A pre-filled, static trail from explicit world-space `points`, newest
    /// last. The ring is sized exactly to the points and treated as fully live,
    /// so the renderer draws all of them; `max_time`/`max_length` (`0.0` to
    /// disable) drive the age/length color gradient in the shader.
    pub fn from_points(points: Vec<TrailPoint>, max_time: f32, max_length: f32) -> Self {
        let len = points.len() as u32;
        let last = points.last();
        Self {
            header: TrailHeader {
                head: len.saturating_sub(1),
                length: len,
                capacity: len,
                current_time: last.map_or(0.0, |p| p.time),
                current_length: last.map_or(0.0, |p| p.length),
                max_time,
                max_length,
                ..default()
            },
            cpu_data: Arc::new(points),
        }
    }
}

/// Cheap clone (`cpu_data` is an [`Arc`]) into the render world.
impl ExtractComponent for TrailData {
    type QueryData = &'static TrailData;
    type QueryFilter = ();
    type Out = TrailData;

    fn extract_component(
        item: bevy::ecs::query::QueryItem<'_, '_, Self::QueryData>,
    ) -> Option<Self::Out> {
        Some(item.clone())
    }
}
