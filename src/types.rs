//! Core data types for trail rendering.

use bevy::prelude::*;
use std::fmt;

/// Per-trail metadata that affects rendering but doesn't change per-vertex.
#[derive(Clone, Debug)]
pub struct TrailMetadata {
    /// Base width scalar applied to all points.
    pub base_width: f32,
    /// How much width tapers toward the oldest trail point (0.0 = no taper, 1.0 = to point).
    pub taper_factor: f32,
    /// Custom metadata slot 0 (e.g., owner player ID).
    pub custom_0: Vec4,
    /// Custom metadata slot 1 (e.g., gameplay state flags).
    pub custom_1: Vec4,
}

impl Default for TrailMetadata {
    fn default() -> Self {
        Self {
            base_width: 0.2,
            taper_factor: 0.3,
            custom_0: Vec4::ZERO,
            custom_1: Vec4::ZERO,
        }
    }
}

/// A single point in a trail, appended each simulation tick.
#[derive(Copy, Clone, Debug)]
pub struct TrailPoint {
    /// World position of the point.
    pub position: Vec3,
    /// Velocity at this point (for shader-side calculations).
    pub velocity: Vec3,
    /// Width multiplier for this specific point (local override).
    pub width: f32,
    /// RGBA color for this point (linear color space).
    pub color: Vec4,
    /// Time this point was spawned (for lifetime/fade calculations).
    pub spawn_time: f32,
    /// Cumulative arc length along the trail up to this point.
    pub cumulative_length: f32,
}

impl Default for TrailPoint {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            width: 1.0,
            color: Vec4::ONE,
            spawn_time: 0.0,
            cumulative_length: 0.0,
        }
    }
}

impl fmt::Display for TrailPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TrailPoint {{ pos: ({:.2}, {:.2}, {:.2}), w: {:.2}, len: {:.2} }}",
            self.position.x, self.position.y, self.position.z, self.width, self.cumulative_length
        )
    }
}

/// Configuration for automatic trail sampling.
#[derive(Clone, Debug)]
pub struct TrailSamplingConfig {
    /// Minimum time between samples (seconds). Prevents excessive sampling.
    pub min_sample_dt: f32,
    /// Distance threshold; sample if traveled this far since last sample.
    pub distance_threshold: f32,
    /// Always sample at least every `max_sample_dt` seconds.
    pub max_sample_dt: f32,
    /// Maximum number of points to store in the trail.
    pub max_points: u32,
    /// Lifetime of each point before removal (seconds).
    pub point_lifetime_secs: f32,
}

impl Default for TrailSamplingConfig {
    fn default() -> Self {
        Self {
            min_sample_dt: 0.01,
            distance_threshold: 0.1,
            max_sample_dt: 0.05,
            max_points: 2000,
            point_lifetime_secs: 3.0,
        }
    }
}
