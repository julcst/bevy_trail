//! High-performance GPU trail renderer for Bevy.
//!
//! This crate provides a reusable trail rendering system that:
//! - Stores trail control points in fixed-capacity ring storage (`capacity`, `head`, `len`)
//! - Uploads ring data to GPU storage buffers
//! - Expands geometry in the vertex shader for camera-facing billboards
//! - Exposes metadata for procedural shading
//!
//! # Quick Start
//!
//! Add [`TrailPlugin`], then spawn anything with a [`Transform`] and a
//! [`TrailEmitter`](emitter::TrailEmitter) — the plugin allocates the GPU
//! buffers, maintains the bounding box for frustum culling, and renders it.
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_trail::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins((DefaultPlugins, TrailPlugin))
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands) {
//!     commands.spawn((Transform::default(), TrailEmitter::default()));
//! }
//! ```

pub mod emitter;
pub mod render;
pub mod types;

use bevy::{camera::primitives::Aabb, prelude::*, render::storage::ShaderStorageBuffer};

use crate::{
    emitter::emit_points_system,
    render::TrailRenderPlugin,
    types::{Trail, TrailData, TrailHeader, TrailPoint, TrailStyle},
};

pub mod prelude {
    pub use crate::{
        emitter::TrailEmitter,
        types::{Trail, TrailPoint, TrailProfile, TrailStyle},
        TrailPlugin,
    };
}

/// System ordering within [`Update`] for the trail lifecycle.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TrailSystems {
    /// Allocates GPU buffers and inserts [`TrailData`] for new trails.
    Init,
    /// Produces new trail points (e.g. from emitters).
    Emit,
    /// Mirrors CPU-side state into GPU buffers and bounding boxes.
    Sync,
}

/// Adds everything needed to spawn and render trails: emission, GPU buffer
/// management, bounding-box upkeep, and the render pipeline.
pub struct TrailPlugin;

impl Plugin for TrailPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TrailRenderPlugin)
            .configure_sets(
                Update,
                (TrailSystems::Init, TrailSystems::Emit, TrailSystems::Sync).chain(),
            )
            .add_systems(Update, init_trails.in_set(TrailSystems::Init))
            .add_systems(Update, emit_points_system.in_set(TrailSystems::Emit))
            .add_systems(
                Update,
                (sync_trail_style, sync_trail_buffers, update_trail_aabb).in_set(TrailSystems::Sync),
            );
    }
}

/// Allocates the GPU storage buffer and inserts the internal [`TrailData`] for
/// any entity that has a [`Trail`] but isn't initialized yet.
fn init_trails(
    mut commands: Commands,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    query: Query<(Entity, &Trail, &TrailStyle), Without<TrailData>>,
) {
    for (entity, trail, style) in &query {
        let capacity = trail.capacity.max(1);
        let cpu_data = vec![TrailPoint::default(); capacity as usize];
        let data = buffers.add(ShaderStorageBuffer::from(cpu_data.clone()));

        commands.entity(entity).insert(TrailData {
            header: TrailHeader {
                capacity,
                max_length: trail.max_length,
                max_time: trail.max_time,
                ..default()
            },
            data,
            cpu_data,
            style: style.clone(),
        });
    }
}

/// Mirrors the user-facing [`TrailStyle`] component into the GPU bind data when
/// it changes, so styles can be animated by simply mutating the component.
fn sync_trail_style(mut query: Query<(&TrailStyle, &mut TrailData), Changed<TrailStyle>>) {
    for (style, mut data) in &mut query {
        data.style = style.clone();
    }
}

/// Uploads CPU ring data to the GPU storage buffer whenever it changes.
fn sync_trail_buffers(
    trails: Query<&TrailData, Changed<TrailData>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    for trail in &trails {
        if let Some(buffer) = buffers.get_mut(&trail.data) {
            buffer.set_data(trail.cpu_data.clone());
        }
    }
}

/// Keeps each trail's [`Aabb`] in sync with its live points so Bevy's normal
/// frustum culling works without the user maintaining a bounding box.
///
/// Trail points are stored in world space, but Bevy culls using a *local-space*
/// `Aabb` transformed by the entity's [`GlobalTransform`]. We therefore pull the
/// world points back into local space before computing the box, and pad it by
/// the trail's half-width to account for the ribbon expanded in the shader.
fn update_trail_aabb(
    mut query: Query<(&GlobalTransform, &TrailData, &mut Aabb), Changed<TrailData>>,
) {
    for (global, trail, mut aabb) in &mut query {
        let header = &trail.header;
        if header.length == 0 || header.capacity == 0 {
            continue;
        }

        let to_local = global.affine().inverse();
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);

        for i in 0..header.length {
            let idx = ((header.head + header.capacity - i) % header.capacity) as usize;
            let local = to_local.transform_point3(trail.cpu_data[idx].position);
            min = min.min(local);
            max = max.max(local);
        }

        let pad = trail.style.start_width.max(trail.style.end_width).max(0.0);
        *aabb = Aabb::from_min_max(min - pad, max + pad);
    }
}
