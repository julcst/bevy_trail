//! GPU-accelerated trail (ribbon) renderer for Bevy.
//!
//! Trail points live in a fixed-capacity ring buffer. Every trail is packed into
//! shared GPU buffers and drawn in one instanced pass per blend mode, with
//! billboard geometry expanded in the vertex shader.
//!
//! A trail is plain data, not a render object, so [`TrailEmitter`](emitter::TrailEmitter)
//! can be added to any entity — even one with its own mesh — without affecting
//! how it renders.
//!
//! # Quick start
//!
//! Add [`TrailPlugin`] and spawn anything with a [`Transform`] and a
//! [`TrailEmitter`](emitter::TrailEmitter):
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
    types::{Trail, TrailData},
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
}

/// Wires up trail emission, state upkeep, and the batched render pipeline.
pub struct TrailPlugin;

impl Plugin for TrailPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TrailRenderPlugin)
            .configure_sets(Update, (TrailSystems::Init, TrailSystems::Emit).chain())
            .add_systems(
                Update,
                (
                    init_trails.in_set(TrailSystems::Init),
                    emit_points_system.in_set(TrailSystems::Emit),
                ),
            );
    }
}

/// Inserts [`TrailData`] for any [`Trail`] that lacks it. No GPU allocation
/// here — the renderer packs all trails into shared buffers each frame.
fn init_trails(mut commands: Commands, query: Query<(Entity, &Trail), Without<TrailData>>) {
    for (entity, trail) in &query {
        commands
            .entity(entity)
            .insert(TrailData::new(trail.capacity, trail.max_length, trail.max_time));
    }
}
