//! Render-world pipeline for trail rendering.
//!
//! This module handles GPU-side rendering of trails, including:
//! - Per-trail GPU buffer management
//! - Vertex shader expansion and billboard facing
//! - Fragment shader procedural effects

use crate::components::Trail;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::PrimitiveTopology;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
    AsBindGroup, Face, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::render::storage::ShaderStorageBuffer;
use bevy::shader::ShaderRef;
use std::collections::HashMap;

pub struct TrailRenderPlugin;

impl Plugin for TrailRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TrailMaterial>::default())
            .init_resource::<TrailStripMeshCache>()
            .add_systems(Startup, preload_trail_shaders)
            .add_systems(FixedUpdate, (attach_trail_render_entities, upload_trail_buffers));
    }
}

#[derive(Resource)]
struct TrailShaderHandles {
    #[allow(dead_code)]
    common: Handle<Shader>,
    #[allow(dead_code)]
    ring: Handle<Shader>,
}

fn preload_trail_shaders(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(TrailShaderHandles {
        common: asset_server.load("shaders/trail/trail_common.wgsl"),
        ring: asset_server.load("shaders/trail/trail_ring.wgsl"),
    });
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TrailMaterial {
    #[uniform(0)]
    pub uniforms: TrailUniforms,
    #[storage(1, read_only)]
    pub points: Handle<ShaderStorageBuffer>,
}

impl Material for TrailMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/trail/trail_ring.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/trail/trail_ring.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Triangle strips flip winding every triangle; disable face culling.
        descriptor.primitive.cull_mode = None::<Face>;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct TrailUniforms {
    pub ring_state: UVec4,
    pub style: Vec4,
    pub custom_a: Vec4,
    pub custom_b: Vec4,
}

impl TrailUniforms {
    pub fn from_trail(trail: &Trail) -> Self {
        Self {
            ring_state: UVec4::new(trail.head, trail.len, trail.capacity, 0),
            style: Vec4::new(
                trail.metadata.base_width,
                trail.metadata.taper_factor,
                0.0,
                0.0,
            ),
            custom_a: trail.metadata.custom_0,
            custom_b: trail.metadata.custom_1,
        }
    }
}

#[derive(Resource, Default)]
pub struct TrailStripMeshCache {
    by_capacity: HashMap<u32, Handle<Mesh>>,
}

pub fn trail_strip_mesh_for_capacity(
    capacity: u32,
    meshes: &mut Assets<Mesh>,
    cache: &mut TrailStripMeshCache,
) -> Handle<Mesh> {
    if let Some(existing) = cache.by_capacity.get(&capacity) {
        return existing.clone();
    }

    let vertex_count = (capacity * 2) as usize;
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleStrip,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![Vec3::ZERO; vertex_count]);

    let handle = meshes.add(mesh);
    cache.by_capacity.insert(capacity, handle.clone());
    handle
}

pub fn pack_points_for_gpu(trail: &Trail) -> Vec<[f32; 4]> {
    // 3 vec4 values per point:
    // [pos.xyz, width], [color.rgba], [velocity.xyz, cumulative_length]
    let mut out = vec![[0.0; 4]; (trail.capacity as usize) * 3];
    for i in 0..trail.capacity as usize {
        let p = trail.points[i];
        out[i * 3] = [p.position.x, p.position.y, p.position.z, p.width];
        out[i * 3 + 1] = [p.color.x, p.color.y, p.color.z, p.color.w];
        out[i * 3 + 2] = [
            p.velocity.x,
            p.velocity.y,
            p.velocity.z,
            p.cumulative_length,
        ];
    }
    out
}

fn attach_trail_render_entities(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cache: ResMut<TrailStripMeshCache>,
    trails: Query<(Entity, &Trail), Added<Trail>>,
) {
    for (entity, trail) in &trails {
        let mesh = trail_strip_mesh_for_capacity(trail.capacity, &mut meshes, &mut cache);
        let render_entity = commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(trail.material.clone()),
                Transform::IDENTITY,
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                NoFrustumCulling,
            ))
            .id();
        commands.entity(entity).add_child(render_entity);
    }
}

fn upload_trail_buffers(
    mut trails: Query<&mut Trail>,
    mut materials: ResMut<Assets<TrailMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    for mut trail in &mut trails {
        if !trail.dirty {
            continue;
        }

        if let Some(material) = materials.get_mut(&trail.material) {
            material.uniforms = TrailUniforms::from_trail(&trail);
        }

        if let Some(buffer) = buffers.get_mut(&trail.gpu_buffer) {
            buffer.set_data(pack_points_for_gpu(&trail));
        }

        trail.dirty = false;
    }
}

impl TrailUniforms {
    pub fn for_config(capacity: u32, metadata: &crate::types::TrailMetadata) -> Self {
        Self {
            ring_state: UVec4::new(0, 0, capacity, 0),
            style: Vec4::new(metadata.base_width, metadata.taper_factor, 0.0, 0.0),
            custom_a: metadata.custom_0,
            custom_b: metadata.custom_1,
        }
    }
}
