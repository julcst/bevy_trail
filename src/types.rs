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
#[repr(u32)]
pub enum TrailProfile {
    Flat,
    Smooth,
    Triangle,
}

#[derive(Clone, Debug, ShaderType)]
pub struct TrailStyle {
    pub start_color: LinearRgba,
    pub end_color: LinearRgba,
    pub start_width: f32,
    pub end_width: f32,
    pub profile: u32, // TODO: Use enum
}

impl Default for TrailStyle {
    fn default() -> Self {
        Self {
            start_color: LinearRgba::WHITE,
            end_color: LinearRgba::WHITE,
            start_width: 1.0,
            end_width: 0.0,
            profile: 0,
        }
    }
}

#[derive(Clone, Debug, ShaderType, Default)]
pub struct TrailPoint {
    pub position: Vec3,
    /// Time along trail
    pub time: f32,
    pub custom: Vec3,
    /// Distance along trail
    pub length: f32,
}

#[derive(Clone, Debug, ShaderType)]
pub struct TrailHeader {
    /// Index of the next point to be overwritten
    pub head: u32,
    pub length: u32,
    pub capacity: u32,
    pub current_time: f32,
    pub max_time: f32,
    pub current_length: f32,
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

#[derive(AsBindGroup, Clone, Asset, Debug, TypePath, Component, ExtractComponent, Default)]
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
