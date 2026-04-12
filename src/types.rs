//! Core data types for trail rendering.

use bevy::{
    camera::visibility::{self, VisibilityClass},
    prelude::*,
    render::{
        extract_component::ExtractComponent,
        render_resource::{AsBindGroup, ShaderType},
        storage::ShaderStorageBuffer,
    },
};

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
    pub t: f32,
}

impl Default for TrailPoint {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            width: 0.0,
            color: Vec4::ZERO,
            velocity: Vec3::ZERO,
            t: 0.0,
        }
    }
}

#[derive(Clone, Debug, ShaderType)]
pub struct TrailHeader {
    /// Index of the next point to be overwritten
    pub head: u32,
    pub length: u32,
    pub capacity: u32,
}

#[derive(AsBindGroup, Clone, Asset, Debug, TypePath, Component, ExtractComponent)]
#[require(VisibilityClass)]
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
