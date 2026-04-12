//! Core data types for trail rendering.

use bevy::{prelude::*, render::render_resource::ShaderType};

#[derive(Clone, Debug)]
pub enum TrailProfile {
    Flat,
    Smooth,
    Triangle,
}

#[derive(Clone, Debug, ShaderType)]
pub struct TrailStyle {
    pub taper: f32,
    pub fade: f32,
    pub profile: u32,
}

#[derive(Clone, Debug, ShaderType)]
pub struct TrailPoint {
    pub position: Vec3,
    pub width: f32,
    pub color: Vec4,
    pub velocity: Vec3,
    pub length: f32,
}

#[derive(Clone, Debug, ShaderType)]
pub struct TrailSampling {
    /// Distance threshold; sample if traveled this far since last sample.
    pub distance_threshold: f32,
    /// Maximum number of points to store in the trail.
    pub max_points: u32,
    pub max_length: f32,
}

#[derive(Clone, Debug, ShaderType)]
pub struct TrailUniforms {
    pub head: u32,
    pub length: u32,
    pub capacity: u32,
}

#[derive(Clone, Debug, ShaderType)]
pub struct Vertex {
    pub pos: Vec3,
}
