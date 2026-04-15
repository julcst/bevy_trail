//! Demonstrates how to enqueue custom draw commands in a render phase.
//!
//! This example shows how to use the built-in
//! [`bevy_render::render_phase::BinnedRenderPhase`] functionality with a
//! custom [`RenderCommand`] to allow inserting arbitrary GPU drawing logic
//! into Bevy's pipeline. This is not the only way to add custom rendering code
//! into Bevy—render nodes are another, lower-level method—but it does allow
//! for better reuse of parts of Bevy's built-in mesh rendering logic.

use crate::types::{TrailData, TrailHeader, TrailPoint, TrailStyle};
use bevy::{
    camera::visibility::{self, VisibilityClass},
    core_pipeline::core_3d::{Opaque3d, Opaque3dBatchSetKey, Opaque3dBinKey, CORE_3D_DEPTH_FORMAT},
    ecs::{
        change_detection::Tick,
        query::ROQueryItem,
        system::{lifetimeless::Read, StaticSystemParam, SystemParamItem},
    },
    mesh::PrimitiveTopology,
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_phase::{
            AddRenderCommand, BinnedRenderPhaseType, DrawFunctions, InputUniformIndex, PhaseItem,
            RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
            ViewBinnedRenderPhases,
        },
        render_resource::{
            AsBindGroup, BindGroup, Canonical, ColorTargetState, ColorWrites, CompareFunction,
            DepthStencilState, FragmentState, PipelineCache, PolygonMode, PrimitiveState,
            RenderPipeline, RenderPipelineDescriptor, Specializer, SpecializerKey, TextureFormat,
            Variants, VertexState,
        },
        renderer::RenderDevice,
        storage::ShaderStorageBuffer,
        view::{ExtractedView, RenderVisibleEntities},
        Render, RenderApp, RenderSystems,
    },
};

/// A [`RenderCommand`] that binds the vertex and index buffers and issues the
/// draw command for our custom phase item.
struct DrawTrail;

impl<P> RenderCommand<P> for DrawTrail
where
    P: PhaseItem,
{
    type Param = ();

    type ViewQuery = ();

    type ItemQuery = Read<GpuTrail>;

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        bind_group: Option<&'w GpuTrail>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(bg) = bind_group else {
            return RenderCommandResult::Failure("BindGroup missing");
        };

        pass.set_bind_group(0, &bg.value, &[]);
        pass.draw(0..bg.vertex_count, 0..1);

        RenderCommandResult::Success
    }
}

#[derive(Component)]
pub struct GpuTrail {
    pub value: BindGroup,
    pub vertex_count: u32,
}

// TODO: only rebuild bind groups when the data changes
fn prepare_trail_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    mut param: StaticSystemParam<<TrailData as AsBindGroup>::Param>,
    // Query extracted trail data and refresh bind groups each frame.
    query: Query<(Entity, &TrailData), Changed<TrailData>>,
) {
    for (entity, trail) in query.iter() {
        let layout_descriptor = TrailData::bind_group_layout_descriptor(&render_device);

        if let Ok(prepared) = trail.as_bind_group(
            &layout_descriptor,
            &render_device,
            &pipeline_cache,
            &mut param,
        ) {
            commands.entity(entity).insert(GpuTrail {
                value: prepared.bind_group,
                vertex_count: trail.header.length * 2, // 2 vertices per point
            });
        }
    }
}

/// The custom draw commands that Bevy executes for each entity we enqueue into
/// the render phase.
type DrawTrailCommands = (
    SetItemPipeline,
    // SetMeshViewBindGroup<0>,
    DrawTrail,
);

pub struct TrailRenderPlugin;

impl Plugin for TrailRenderPlugin {
    fn build(&self, app: &mut App) {
        // Main World
        app.init_asset::<TrailData>()
            .add_plugins((ExtractComponentPlugin::<TrailData>::default(),));

        // Render World
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_render_command::<Opaque3d, DrawTrailCommands>()
            .add_systems(
                Render,
                (
                    prepare_trail_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                    queue_custom_phase_item.in_set(RenderSystems::Queue),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        // CustomPhasePipeline needs RenderDevice to be created, which doesn't happen until App::run
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<TrailPipeline>();
    }
}

/// A render-world system that enqueues the entity with custom rendering into
/// the opaque render phases of each view.
fn queue_custom_phase_item(
    pipeline_cache: Res<PipelineCache>,
    mut pipeline: ResMut<TrailPipeline>,
    mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    views: Query<(&ExtractedView, &RenderVisibleEntities, &Msaa)>,
    mut next_tick: Local<Tick>,
) {
    let draw_custom_phase_item = opaque_draw_functions.read().id::<DrawTrailCommands>();

    // Render phases are per-view, so we need to iterate over all views so that
    // the entity appears in them. (In this example, we have only one view, but
    // it's good practice to loop over all views anyway.)
    for (view, view_visible_entities, msaa) in views.iter() {
        let Some(opaque_phase) = opaque_render_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        // Find all the custom rendered entities that are visible from this
        // view.
        for &entity in view_visible_entities.get::<TrailData>().iter() {
            // Ordinarily, the [`SpecializedRenderPipeline::Key`] would contain
            // some per-view settings, such as whether the view is HDR, but for
            // simplicity's sake we simply hard-code the view's characteristics,
            // with the exception of number of MSAA samples.
            let Ok(pipeline_id) = pipeline
                .variants
                .specialize(&pipeline_cache, TrailPipelineKey { msaa: *msaa })
            else {
                continue;
            };

            // Bump the change tick in order to force Bevy to rebuild the bin.
            let this_tick = next_tick.get() + 1;
            next_tick.set(this_tick);

            // Add the custom render item. We use the
            // [`BinnedRenderPhaseType::NonMesh`] type to skip the special
            // handling that Bevy has for meshes (preprocessing, indirect
            // draws, etc.)
            //
            // The asset ID is arbitrary; we simply use [`AssetId::invalid`],
            // but you can use anything you like. Note that the asset ID need
            // not be the ID of a [`Mesh`].
            opaque_phase.add(
                Opaque3dBatchSetKey {
                    draw_function: draw_custom_phase_item,
                    pipeline: pipeline_id,
                    material_bind_group_index: None,
                    lightmap_slab: None,
                    vertex_slab: default(),
                    index_slab: None,
                },
                Opaque3dBinKey {
                    asset_id: AssetId::<Mesh>::invalid().untyped(),
                },
                entity,
                InputUniformIndex::default(),
                BinnedRenderPhaseType::NonMesh,
                *next_tick,
            );
        }
    }
}

#[derive(Resource)]
struct TrailPipeline {
    /// the `variants` collection holds onto the shader handle through the base descriptor
    variants: Variants<RenderPipeline, TrailPipelineSpecializer>,
}

impl FromWorld for TrailPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let shader = asset_server.load("shaders/trail_drawing.wgsl");
        let render_device = world.resource::<RenderDevice>();
        let material_layout = TrailData::bind_group_layout_descriptor(render_device);

        let base_descriptor = RenderPipelineDescriptor {
            label: Some("custom render pipeline".into()),
            layout: vec![material_layout],
            vertex: VertexState {
                shader: shader.clone(),
                // No buffers
                ..default()
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                cull_mode: None,
                // polygon_mode: PolygonMode::Line,
                ..default()
            },
            fragment: Some(FragmentState {
                shader: shader.clone(),
                targets: vec![Some(ColorTargetState {
                    // Ordinarily, you'd want to check whether the view has the
                    // HDR format and substitute the appropriate texture format
                    // here, but we omit that for simplicity.
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            // Note that if your view has no depth buffer this will need to be
            // changed.
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: default(),
                bias: default(),
            }),
            ..default()
        };

        let variants = Variants::new(TrailPipelineSpecializer, base_descriptor);

        Self { variants }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, SpecializerKey)]
struct TrailPipelineKey {
    msaa: Msaa,
}

struct TrailPipelineSpecializer;

impl Specializer<RenderPipeline> for TrailPipelineSpecializer {
    type Key = TrailPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        descriptor: &mut RenderPipelineDescriptor,
    ) -> Result<Canonical<Self::Key>, BevyError> {
        descriptor.multisample.count = key.msaa.samples();
        Ok(key)
    }
}
