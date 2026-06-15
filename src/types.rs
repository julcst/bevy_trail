//! Core data types for trail rendering.

use std::sync::Arc;

use bevy::{
    prelude::*,
    render::{extract_component::ExtractComponent, render_resource::ShaderType},
};

/// Cross-section shape of the trail ribbon.
///
/// The profile modulates the ribbon's alpha across its width, so it is most
/// visible with an alpha-aware [`TrailRenderMode`] (additive or transparent):
/// - [`Flat`](Self::Flat) keeps a constant, hard-edged ribbon.
/// - [`Smooth`](Self::Smooth) fades the edges with a rounded falloff, giving a
///   soft, tube-like look.
/// - [`Triangle`](Self::Triangle) fades linearly to the edges, peaking in the
///   middle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum TrailProfile {
    #[default]
    Flat = 0,
    Smooth = 1,
    Triangle = 2,
}

/// How a trail's pixels are blended into the frame.
///
/// This is a [`Component`]; [`Trail`] inserts a default one via `#[require]`.
/// Mutate it at runtime to switch how the trail composites with the scene.
#[derive(Component, ExtractComponent, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TrailRenderMode {
    /// Opaque: alpha is ignored and the trail overwrites whatever is behind it.
    #[default]
    Opaque,
    /// Additive: the trail's color is added to the frame, scaled by its alpha.
    /// Great for glowing, energetic effects; order-independent.
    Additive,
    /// Straight alpha blending: the trail's alpha controls how much it lets the
    /// background show through.
    Transparent,
}

/// Appearance of a trail.
///
/// This is a [`Component`] in its own right, so it can be queried and mutated
/// independently of the rest of the trail (e.g. to animate colors or width at
/// runtime). [`Trail`] inserts a default one for you via `#[require]`. The
/// renderer reads it straight off each trail entity (it is an [`ExtractComponent`]),
/// so mutating it animates the trail with no extra bookkeeping.
#[derive(Component, ExtractComponent, Clone, Debug, ShaderType)]
pub struct TrailStyle {
    pub start_color: LinearRgba,
    pub end_color: LinearRgba,
    pub start_width: f32,
    pub end_width: f32,
    /// Cross-section shape, stored as the `u32` repr of a [`TrailProfile`] so it
    /// can travel into the shader uniform. Use [`with_profile`](Self::with_profile)
    /// or assign `profile as u32` to set it.
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
    /// Time along trail
    pub time: f32,
    pub custom: Vec3,
    /// Distance along trail
    pub length: f32,
}

/// User-facing configuration for a trail.
///
/// Add it to **any** entity that has a [`Transform`] — including one that already
/// has its own [`Mesh3d`](bevy::prelude::Mesh3d), camera, or gameplay components.
/// A trail is *not* a render object: its points are sampled in world space and
/// drawn by a single global batched pass, so attaching one never interferes with
/// the entity's own rendering. To have the trail follow the entity automatically,
/// also add a [`TrailEmitter`](crate::emitter::TrailEmitter).
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
    /// Base index of this trail's points within the batched `points` storage
    /// buffer. Set by the renderer while building the draw batch; `0` otherwise.
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

/// The renderable state of a trail: its world-space ring buffer and the header
/// describing the live segment.
///
/// This is a plain data component, **not** a render object — it carries no
/// `Visibility`, `Aabb`, or visibility class. Every frame the renderer simply
/// collects the [`TrailData`], [`TrailStyle`], and [`TrailRenderMode`] of all
/// trails and packs them into shared GPU buffers for one batched instanced draw
/// (see [`crate::render`]), so a trail adds no per-entity draw, bind group, or
/// culling bookkeeping.
///
/// The plugin inserts and maintains this automatically from [`Trail`]; you
/// normally never construct it yourself. It is public only for advanced,
/// low-level use (e.g. feeding a pre-baked, static trail via [`from_points`]),
/// in which case [`TrailStyle`] and [`TrailRenderMode`] are supplied for you via
/// `#[require]`.
///
/// # Invariant
///
/// `cpu_data.len() == header.capacity` and `header.length <= header.capacity`.
/// The constructors establish this and the emitter preserves it; the renderer
/// relies on it. Build instances with [`new`] or [`from_points`] rather than the
/// struct literal so you can't violate it by accident.
///
/// [`new`]: Self::new
/// [`from_points`]: Self::from_points
#[derive(Clone, Component)]
#[require(TrailStyle, TrailRenderMode)]
pub struct TrailData {
    pub header: TrailHeader,
    /// Live ring-buffer points, in **world space**. The renderer concatenates
    /// these across all trails into one shared GPU buffer and draws them in a
    /// single instanced draw call (see [`crate::render`]).
    ///
    /// Held behind an [`Arc`] so extracting the trail into the render world each
    /// frame is a cheap pointer clone; the emitter mutates it via
    /// [`Arc::make_mut`], which only deep-copies when a trail actually changes.
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

/// Extract [`TrailData`] into the render world: a cheap clone, since `cpu_data`
/// is an [`Arc`]. The renderer packs every trail's points into one shared GPU
/// buffer and draws them in a single instanced call (see [`crate::render`]),
/// avoiding both per-frame buffer reallocation and one draw call per trail.
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
