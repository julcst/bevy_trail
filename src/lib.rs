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
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_trail::prelude::*;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(TrailPlugin)
//!     .run();
//! ```

pub mod emitter;
pub mod render;
pub mod types;

pub mod prelude {
    
}
