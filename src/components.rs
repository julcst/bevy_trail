//! ECS components for trail entities.

use crate::render::TrailMaterial;
use crate::types::{TrailMetadata, TrailPoint, TrailSamplingConfig};
use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;

/// Component attached to entities that should have trails rendered.
///
/// This component holds the trail history and rendering configuration.
/// The entity also needs a `GlobalTransform` to determine trail positions.
#[derive(Component, Debug)]
pub struct Trail {
    /// Fixed-capacity ring storage for control points.
    pub points: Vec<TrailPoint>,
    /// Rendering metadata (width, taper, blend mode, custom data).
    pub metadata: TrailMetadata,
    /// Maximum ring capacity.
    pub capacity: u32,
    /// Next insertion index in the ring.
    pub head: u32,
    /// Number of valid points currently in the ring.
    pub len: u32,
    /// Last position where a point was sampled (for distance threshold).
    pub(crate) last_sample_position: Vec3,
    /// Last time a point was sampled.
    pub(crate) last_sample_time: f32,
    /// Total simulation time for this trail.
    pub elapsed_time: f32,
    /// Marks whether GPU data needs upload this frame.
    pub dirty: bool,
    /// Storage buffer used by the trail shader.
    pub gpu_buffer: Handle<ShaderStorageBuffer>,
    /// Material used by the trail shader.
    pub material: Handle<TrailMaterial>,
}

impl Trail {
    /// Create a new trail with the given sampling config and metadata.
    pub fn new(config: &TrailSamplingConfig, metadata: TrailMetadata) -> Self {
        Self::new_with_gpu(config, metadata, Handle::default(), Handle::default())
    }

    /// Create a new trail with explicit GPU resources.
    pub fn new_with_gpu(
        config: &TrailSamplingConfig,
        metadata: TrailMetadata,
        gpu_buffer: Handle<ShaderStorageBuffer>,
        material: Handle<TrailMaterial>,
    ) -> Self {
        assert!(
            config.max_points >= 2,
            "TrailSamplingConfig.max_points must be >= 2"
        );
        let capacity = config.max_points;
        Self {
            points: vec![TrailPoint::default(); capacity as usize],
            metadata,
            capacity,
            head: 0,
            len: 0,
            last_sample_position: Vec3::ZERO,
            last_sample_time: 0.0,
            elapsed_time: 0.0,
            dirty: true,
            gpu_buffer,
            material,
        }
    }

    /// Attempt to sample a new point and add it to the trail.
    ///
    /// Returns `true` if a point was added.
    pub fn try_sample(
        &mut self,
        position: Vec3,
        velocity: Vec3,
        config: &TrailSamplingConfig,
        delta: f32,
    ) -> bool {
        self.elapsed_time += delta;

        let time_since_last = self.elapsed_time - self.last_sample_time;
        let distance_since_last = (position - self.last_sample_position).length();

        let should_sample = time_since_last >= config.min_sample_dt
            && (distance_since_last >= config.distance_threshold
                || time_since_last >= config.max_sample_dt);

        if should_sample {
            let prev = self.last_point().copied();
            let cumulative_length = if let Some(p) = prev {
                p.cumulative_length + distance_since_last
            } else {
                0.0
            };

            let point = TrailPoint {
                position,
                velocity,
                width: self.metadata.base_width,
                color: Vec4::ONE, // Default white; can be overridden per-emitter
                spawn_time: self.elapsed_time,
                cumulative_length,
            };

            self.push_point(point);
            self.last_sample_position = position;
            self.last_sample_time = self.elapsed_time;
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Prune old points based on lifetime and update internal time.
    pub fn update(&mut self, config: &TrailSamplingConfig, _delta: f32) {
        let lifetime = config.point_lifetime_secs;
        let mut removed_any = false;
        while self.len > 0 {
            let oldest_idx = self.oldest_index();
            let oldest = self.points[oldest_idx as usize];
            if self.elapsed_time - oldest.spawn_time <= lifetime {
                break;
            }
            self.len -= 1;
            removed_any = true;
        }
        if removed_any {
            self.dirty = true;
        }
    }

    /// Get the current number of points in the trail.
    pub fn point_count(&self) -> u32 {
        self.len
    }

    /// Get a point by logical index, where 0 is oldest and len-1 is newest.
    pub fn point_by_logical(&self, logical: u32) -> TrailPoint {
        let physical = (self.oldest_index() + logical) % self.capacity;
        self.points[physical as usize]
    }

    fn oldest_index(&self) -> u32 {
        (self.head + self.capacity - self.len) % self.capacity
    }

    fn last_point(&self) -> Option<&TrailPoint> {
        if self.len == 0 {
            return None;
        }
        let newest = (self.head + self.capacity - 1) % self.capacity;
        self.points.get(newest as usize)
    }

    fn push_point(&mut self, point: TrailPoint) {
        self.points[self.head as usize] = point;
        self.head = (self.head + 1) % self.capacity;
        self.len = (self.len + 1).min(self.capacity);
    }
}

/// Configuration for an entity that automatically emits trail points.
///
/// Attach this to an entity along with a `GlobalTransform`, and the trail
/// system will automatically sample positions and create/update a `Trail` component.
#[derive(Component, Clone)]
pub struct TrailEmitterConfig {
    /// Sampling configuration for the trail.
    pub sampling: TrailSamplingConfig,
    /// Rendering metadata for the trail.
    pub metadata: TrailMetadata,
    /// Optional callback to override point color based on gameplay state.
    /// Called for each sampled point; receives (position, velocity) and returns color override.
    pub color_fn: Option<std::sync::Arc<dyn Fn(Vec3, Vec3) -> Vec4 + Send + Sync>>,
}

impl TrailEmitterConfig {
    /// Create a new emitter config with default sampling and rendering settings.
    pub fn new() -> Self {
        Self {
            sampling: TrailSamplingConfig::default(),
            metadata: TrailMetadata::default(),
            color_fn: None,
        }
    }

    /// Set the sampling configuration.
    pub fn with_sampling(mut self, sampling: TrailSamplingConfig) -> Self {
        self.sampling = sampling;
        self
    }

    /// Set the rendering metadata.
    pub fn with_metadata(mut self, metadata: TrailMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set a custom color function.
    pub fn with_color_fn(mut self, f: impl Fn(Vec3, Vec3) -> Vec4 + Send + Sync + 'static) -> Self {
        self.color_fn = Some(std::sync::Arc::new(f));
        self
    }
}

impl Default for TrailEmitterConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Marker component for entities that should emit trails.
///
/// When an entity has both `TrailEmitter` and a `GlobalTransform`, the trail system
/// will automatically manage the `Trail` component and sample positions each frame.
#[derive(Component, Debug)]
pub struct TrailEmitter;
