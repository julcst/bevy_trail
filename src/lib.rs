//! High-performance GPU trail renderer for Bevy.
//!
//! This crate provides a reusable trail rendering system that:
//! - Stores trail control points in fixed-capacity ring storage (`capacity`, `head`, `len`)
//! - Packs every trail into shared GPU buffers drawn by a single instanced pass
//! - Expands geometry in the vertex shader for camera-facing billboards
//! - Exposes metadata for procedural shading
//!
//! A trail is plain data, not a render object, so [`TrailEmitter`](emitter::TrailEmitter)
//! can be added to any existing entity (even one with its own mesh) without
//! interfering with how that entity renders.
//!
//! # Quick Start
//!
//! Add [`TrailPlugin`], then spawn anything with a [`Transform`] and a
//! [`TrailEmitter`](emitter::TrailEmitter) — the plugin samples its path and
//! renders it as part of the global trail batch.
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

use bevy::prelude::*;

use crate::{
    emitter::emit_points_system,
    render::TrailRenderPlugin,
    types::{Trail, TrailData, TrailHeader, TrailPoint, TrailStyle},
};

pub mod prelude {
    pub use crate::{
        emitter::TrailEmitter,
        types::{Trail, TrailPoint, TrailProfile, TrailRenderMode, TrailStyle},
        TrailPlugin,
    };
}

/// System ordering within [`Update`] for the trail lifecycle.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TrailSystems {
    /// Inserts the internal [`TrailData`] for new trails.
    Init,
    /// Produces new trail points (e.g. from emitters).
    Emit,
    /// Mirrors user-facing component state into [`TrailData`].
    Sync,
}

/// Adds everything needed to spawn and render trails: emission, trail-state
/// upkeep, and the batched render pipeline.
pub struct TrailPlugin;

impl Plugin for TrailPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TrailRenderPlugin)
            .configure_sets(
                Update,
                (TrailSystems::Init, TrailSystems::Emit, TrailSystems::Sync).chain(),
            )
            .add_systems(
                Update,
                (
                    init_trails.in_set(TrailSystems::Init),
                    emit_points_system.in_set(TrailSystems::Emit),
                    sync_trail_style.in_set(TrailSystems::Sync),
                ),
            );
    }
}

/// Inserts the internal [`TrailData`] for any entity that has a [`Trail`] but
/// isn't initialized yet. No GPU buffers are allocated here: the renderer packs
/// every trail's points into shared batched buffers each frame (see
/// [`crate::render`]).
fn init_trails(
    mut commands: Commands,
    query: Query<(Entity, &Trail, &TrailStyle), Without<TrailData>>,
) {
    for (entity, trail, style) in &query {
        let capacity = trail.capacity.max(1);
        let cpu_data = vec![TrailPoint::default(); capacity as usize];

        commands.entity(entity).insert(TrailData {
            header: TrailHeader {
                capacity,
                max_length: trail.max_length,
                max_time: trail.max_time,
                ..default()
            },
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
