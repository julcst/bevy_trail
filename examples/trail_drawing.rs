//! Demonstrates how to enqueue custom draw commands in a render phase.
//!
//! This example shows how to use the built-in
//! [`bevy_render::render_phase::BinnedRenderPhase`] functionality with a
//! custom [`RenderCommand`] to allow inserting arbitrary GPU drawing logic
//! into Bevy's pipeline. This is not the only way to add custom rendering code
//! into Bevy—render nodes are another, lower-level method—but it does allow
//! for better reuse of parts of Bevy's built-in mesh rendering logic.

use bevy::{
    camera::{
        primitives::Aabb,
        visibility::{self, VisibilityClass},
    },
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
            DepthStencilState, FragmentState, PipelineCache, PrimitiveState, RenderPipeline,
            RenderPipelineDescriptor, Specializer, SpecializerKey, TextureFormat, Variants,
            VertexState,
        },
        renderer::RenderDevice,
        storage::ShaderStorageBuffer,
        view::{ExtractedView, RenderVisibleEntities},
        Render, RenderApp, RenderSystems,
    },
    shader::ShaderRef,
};
use bevy_trail::types::{TrailPoint, TrailStyle, TrailUniforms};

/// The entry point.
fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, CustomTrailRenderPlugin))
        .add_systems(Startup, setup);
    app.run();
}

/// A marker component that represents an entity that is to be rendered using
/// our custom phase item.
///
/// Note the [`ExtractComponent`] trait implementation: this is necessary to
/// tell Bevy that this object should be pulled into the render world. Also note
/// the `on_add` hook, which is needed to tell Bevy's `check_visibility` system
/// that entities with this component need to be examined for visibility.
#[derive(Clone, Component, ExtractComponent)]
#[require(VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<CustomRenderedEntity>)]
struct CustomRenderedEntity;

/// A [`RenderCommand`] that binds the vertex and index buffers and issues the
/// draw command for our custom phase item.
struct DrawCustomPhaseItem;

impl<P> RenderCommand<P> for DrawCustomPhaseItem
where
    P: PhaseItem,
{
    type Param = ();

    type ViewQuery = ();

    type ItemQuery = Read<CustomTrailBindGroup>;

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        bind_group: Option<&'w CustomTrailBindGroup>,
        _custom_phase_item_buffers: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(bg) = bind_group else {
            return RenderCommandResult::Failure("BindGroup missing".into());
        };

        pass.set_bind_group(0, &bg.value, &[]);
        pass.draw(0..6, 0..1);

        RenderCommandResult::Success
    }
}

/// The GPU vertex and index buffers for our custom phase item.
///
/// As the custom phase item is a single triangle, these are uploaded once and
/// then left alone.
#[derive(Resource)]
struct CustomPhaseItemBuffers {}

#[derive(AsBindGroup, Clone, Asset, Debug, TypePath, Component, ExtractComponent)]
#[extract_component_filter(With<CustomRenderedEntity>)]
struct CustomMaterial {
    #[uniform(0)]
    trail: TrailUniforms,
    #[storage(1, read_only)]
    trail_points: Handle<ShaderStorageBuffer>,
    #[uniform(2)]
    style: TrailStyle,
}

#[derive(Component)]
pub struct CustomTrailBindGroup {
    pub value: BindGroup,
}

fn prepare_trail_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    mut param: StaticSystemParam<<CustomMaterial as AsBindGroup>::Param>,
    // Query the materials we just extracted
    query: Query<(Entity, &CustomMaterial), Without<CustomTrailBindGroup>>,
) {
    for (entity, material) in query.iter() {
        let layout_descriptor = CustomMaterial::bind_group_layout_descriptor(&render_device);

        if let Ok(prepared) = material.as_bind_group(
            &layout_descriptor,
            &render_device,
            &pipeline_cache,
            &mut param,
        ) {
            commands.entity(entity).insert(CustomTrailBindGroup {
                value: prepared.bind_group,
            });
        }
    }
}

/// The custom draw commands that Bevy executes for each entity we enqueue into
/// the render phase.
type DrawCustomPhaseItemCommands = (
    SetItemPipeline,
    // SetMeshViewBindGroup<0>,
    DrawCustomPhaseItem,
);

pub struct CustomTrailRenderPlugin;

impl Plugin for CustomTrailRenderPlugin {
    fn build(&self, app: &mut App) {
        // Main World
        app.init_asset::<CustomMaterial>().add_plugins((
            ExtractComponentPlugin::<CustomRenderedEntity>::default(),
            ExtractComponentPlugin::<CustomMaterial>::default(),
        ));

        // Render World
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_render_command::<Opaque3d, DrawCustomPhaseItemCommands>()
            .add_systems(
                Render,
                (
                    queue_custom_phase_item.in_set(RenderSystems::Queue),
                    prepare_trail_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        // CustomPhasePipeline needs RenderDevice to be created, which doesn't happen until App::run
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<CustomPhasePipeline>();
    }
}

/// Spawns the objects in the scene.
fn setup(mut commands: Commands, mut buffers: ResMut<Assets<ShaderStorageBuffer>>) {
    let data = [
        TrailPoint {
            position: Vec3::new(-0.5, -0.5, 0.0),
            width: 1.0,
            color: Vec4::X,
            velocity: Vec3::ZERO,
            length: 1.0,
        },
        TrailPoint {
            position: Vec3::new(0.5, -0.5, 0.0),
            width: 1.0,
            color: Vec4::Y,
            velocity: Vec3::ZERO,
            length: 1.0,
        },
        TrailPoint {
            position: Vec3::new(-0.5, 0.5, 0.0),
            width: 1.0,
            color: Vec4::Z,
            velocity: Vec3::ZERO,
            length: 1.0,
        },
    ];

    let trail = TrailUniforms {
        head: 0,
        length: 3,
        capacity: 3,
    };

    let style = TrailStyle {
        taper: 0.5,
        fade: 0.5,
        profile: 0,
    };

    let trail_points = buffers.add(ShaderStorageBuffer::from(data));

    // Spawn a single entity that has custom rendering. It'll be extracted into
    // the render world via [`ExtractComponent`].
    commands.spawn((
        Visibility::default(),
        Transform::default(),
        // This `Aabb` is necessary for the visibility checks to work.
        Aabb {
            center: Vec3A::ZERO,
            half_extents: Vec3A::splat(0.5),
        },
        CustomRenderedEntity,
        CustomMaterial {
            trail,
            trail_points: trail_points.clone(),
            style,
        },
    ));

    // Spawn the camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// A render-world system that enqueues the entity with custom rendering into
/// the opaque render phases of each view.
fn queue_custom_phase_item(
    pipeline_cache: Res<PipelineCache>,
    mut pipeline: ResMut<CustomPhasePipeline>,
    mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    views: Query<(&ExtractedView, &RenderVisibleEntities, &Msaa)>,
    mut next_tick: Local<Tick>,
) {
    let draw_custom_phase_item = opaque_draw_functions
        .read()
        .id::<DrawCustomPhaseItemCommands>();

    // Render phases are per-view, so we need to iterate over all views so that
    // the entity appears in them. (In this example, we have only one view, but
    // it's good practice to loop over all views anyway.)
    for (view, view_visible_entities, msaa) in views.iter() {
        let Some(opaque_phase) = opaque_render_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        // Find all the custom rendered entities that are visible from this
        // view.
        for &entity in view_visible_entities.get::<CustomRenderedEntity>().iter() {
            // Ordinarily, the [`SpecializedRenderPipeline::Key`] would contain
            // some per-view settings, such as whether the view is HDR, but for
            // simplicity's sake we simply hard-code the view's characteristics,
            // with the exception of number of MSAA samples.
            let Ok(pipeline_id) = pipeline
                .variants
                .specialize(&pipeline_cache, CustomPhaseKey(*msaa))
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

struct CustomPhaseSpecializer;

#[derive(Resource)]
struct CustomPhasePipeline {
    /// the `variants` collection holds onto the shader handle through the base descriptor
    variants: Variants<RenderPipeline, CustomPhaseSpecializer>,
}

impl FromWorld for CustomPhasePipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let shader = asset_server.load("shaders/trail_drawing.wgsl");
        let render_device = world.resource::<RenderDevice>();
        let material_layout = CustomMaterial::bind_group_layout_descriptor(render_device);

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

        let variants = Variants::new(CustomPhaseSpecializer, base_descriptor);

        Self { variants }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, SpecializerKey)]
struct CustomPhaseKey(Msaa);

impl Specializer<RenderPipeline> for CustomPhaseSpecializer {
    type Key = CustomPhaseKey;

    fn specialize(
        &self,
        key: Self::Key,
        descriptor: &mut RenderPipelineDescriptor,
    ) -> Result<Canonical<Self::Key>, BevyError> {
        descriptor.multisample.count = key.0.samples();
        Ok(key)
    }
}
