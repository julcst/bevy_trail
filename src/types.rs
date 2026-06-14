//! Core data types for trail rendering.

use bevy::{
    camera::{
        primitives::Aabb,
        visibility::{self, VisibilityClass},
    },
    prelude::*,
    render::{
        extract_component::ExtractComponent,
        render_resource::{AsBindGroup, ShaderType},
        storage::ShaderStorageBuffer,
    },
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
    Normal,
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
/// runtime). [`Trail`] inserts a default one for you via `#[require]`.
#[derive(Component, Clone, Debug, ShaderType)]
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
/// Add this to any entity (together with a [`Transform`]) and the plugin takes
/// care of the rest: allocating the GPU buffers, maintaining the bounding box
/// for frustum culling, and rendering. To have the trail follow the entity
/// automatically, also add a [`TrailEmitter`](crate::emitter::TrailEmitter).
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
#[require(Transform, Visibility, TrailStyle, TrailRenderMode)]
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
        }
    }
}

/// Internal GPU mirror of a trail: the ring buffer, its header, and a copy of
/// the style, bundled for binding.
///
/// The plugin inserts and maintains this automatically from [`Trail`] and
/// [`TrailStyle`]; you normally never construct it yourself. It is public only
/// for advanced, low-level use (e.g. feeding a pre-baked, static trail).
///
/// The frustum-culling components ([`Visibility`], [`Aabb`], [`VisibilityClass`])
/// live here, on the component that is actually rendered, so culling works
/// without any setup from the user.
#[derive(AsBindGroup, Clone, Component, ExtractComponent, Default)]
#[require(Transform, Visibility, Aabb, VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<TrailData>)]
pub struct TrailData {
    #[uniform(0)]
    pub header: TrailHeader,
    #[storage(1, read_only)]
    pub data: Handle<ShaderStorageBuffer>,
    pub cpu_data: Vec<TrailPoint>,
    #[uniform(2)]
    pub style: TrailStyle,
}
