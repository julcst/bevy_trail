//! Batched trail rendering: all trails are packed into shared GPU storage
//! buffers and drawn with a single instanced draw call per blend mode, rather
//! than one draw call + bind group per trail. Geometry is expanded in the
//! vertex shader (camera-facing billboards), so there are no vertex buffers.

use crate::types::{TrailData, TrailHeader, TrailPoint, TrailRenderMode, TrailStyle};
use bevy::{
    core_pipeline::core_3d::{Opaque3d, Opaque3dBatchSetKey, Opaque3dBinKey, CORE_3D_DEPTH_FORMAT},
    ecs::{
        change_detection::Tick,
        query::ROQueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    mesh::PrimitiveTopology,
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_phase::{
            AddRenderCommand, BinnedRenderPhaseType, DrawFunctions, InputUniformIndex, PhaseItem,
            RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
            ViewBinnedRenderPhases,
        },
        render_resource::{
            binding_types::{storage_buffer_read_only_sized, uniform_buffer},
            encase::StorageBuffer as EncaseStorageBuffer,
            BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            BlendComponent, BlendFactor, BlendOperation, BlendState, Buffer, BufferDescriptor,
            BufferUsages, Canonical, ColorTargetState, ColorWrites, CompareFunction,
            DepthStencilState, FragmentState, PipelineCache, PrimitiveState, RenderPipeline,
            RenderPipelineDescriptor, ShaderStages, Specializer, SpecializerKey, TextureFormat,
            Variants, VertexState,
        },
        renderer::{RenderDevice, RenderQueue},
        sync_world::MainEntity,
        view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
        Render, RenderApp, RenderSystems,
    },
};

/// The three blend modes, in [`TrailRenderMode`] discriminant order, so a mode
/// round-trips through `mode as usize` and back.
const MODES: [TrailRenderMode; 3] = [
    TrailRenderMode::Opaque,
    TrailRenderMode::Additive,
    TrailRenderMode::Transparent,
];

/// Per-blend-mode draw anchor; the renderer owns one per [`TrailRenderMode`] as
/// the representative entity for that batch's phase item.
///
/// Mesh-less on purpose: the binned phase caches per `MainEntity` across item
/// kinds, so reusing a real trail entity (which may draw its own mesh) would
/// make the two items fight over one slot and drop both. The mode lives here
/// (not as a [`TrailRenderMode`] component) so anchors stay out of user queries.
#[derive(Component, ExtractComponent, Clone, Copy)]
struct TrailBatchAnchor {
    mode: TrailRenderMode,
}

/// Binds one blend mode's batched buffers and issues a single instanced draw
/// covering every trail in that batch.
struct DrawTrailBatch;

impl<P> RenderCommand<P> for DrawTrailBatch
where
    P: PhaseItem,
{
    type Param = SRes<TrailBatches>;
    type ViewQuery = ();
    // The phase item's entity is the batch anchor; its mode tells us which batch
    // this draw is for.
    type ItemQuery = bevy::ecs::system::lifetimeless::Read<TrailBatchAnchor>;

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        anchor: Option<&'w TrailBatchAnchor>,
        param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(anchor) = anchor else {
            return RenderCommandResult::Skip;
        };
        let batch = &param.into_inner().modes[anchor.mode as usize];

        let Some(bind_group) = batch.bind_group.as_ref() else {
            return RenderCommandResult::Skip;
        };
        if batch.instance_count == 0 || batch.max_verts == 0 {
            return RenderCommandResult::Skip;
        }

        pass.set_bind_group(1, bind_group, &[]);
        pass.draw(0..batch.max_verts, 0..batch.instance_count);

        RenderCommandResult::Success
    }
}

/// A GPU storage buffer that grows on demand and remembers its capacity, so a
/// steady or shrinking trail count doesn't reallocate every frame.
#[derive(Default)]
struct GrowBuffer {
    buffer: Option<Buffer>,
    capacity: u64,
}

/// Per-blend-mode batch of trails: shared GPU buffers, one bind group, and the
/// instanced draw parameters. Buffers are kept across frames and only grown
/// (which forces a bind-group rebuild) when the trail count rises.
#[derive(Default)]
struct GpuBatch {
    headers: GrowBuffer,
    points: GrowBuffer,
    styles: GrowBuffer,
    bind_group: Option<BindGroup>,
    /// Number of trails (= instances) in this batch.
    instance_count: u32,
    /// Vertices per instance: 2 per live ring point, sized to the longest trail.
    max_verts: u32,
}

#[derive(Resource, Default)]
struct TrailBatches {
    /// One batch per blend mode, indexed by `TrailRenderMode as usize`.
    modes: [GpuBatch; 3],
}

/// Whether any trail changed this frame. Extracted to the render world so it can
/// skip the whole repack when nothing changed (e.g. a static scene).
#[derive(Resource, Default, Clone, Copy, ExtractResource)]
struct TrailsChanged(bool);

/// Sets [`TrailsChanged`] if any trail's data, style, or mode changed, or one
/// was spawned (reads as `Changed`) or despawned. O(changes), not O(trails).
fn detect_trail_changes(
    data_changed: Query<(), Changed<TrailData>>,
    style_changed: Query<(), Changed<TrailStyle>>,
    mode_changed: Query<(), Changed<TrailRenderMode>>,
    removed: RemovedComponents<TrailData>,
    mut changed: ResMut<TrailsChanged>,
) {
    changed.0 = !data_changed.is_empty()
        || !style_changed.is_empty()
        || !mode_changed.is_empty()
        || !removed.is_empty();
}

/// Bind group for the view uniform (camera matrices), bound at group 0 so the
/// shader can project world-space points into clip space. Rebuilt each frame.
#[derive(Resource, Default)]
struct TrailViewBindGroup(Option<BindGroup>);

/// Binds the view uniform at group `I` with the per-view dynamic offset.
struct SetTrailViewBindGroup<const I: usize>;

impl<P, const I: usize> RenderCommand<P> for SetTrailViewBindGroup<I>
where
    P: PhaseItem,
{
    type Param = SRes<TrailViewBindGroup>;

    type ViewQuery = bevy::ecs::system::lifetimeless::Read<ViewUniformOffset>;

    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        view_offset: ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<()>,
        param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(bind_group) = param.into_inner().0.as_ref() else {
            return RenderCommandResult::Failure("View bind group missing");
        };

        pass.set_bind_group(I, bind_group, &[view_offset.offset]);

        RenderCommandResult::Success
    }
}

/// Rebuilds the view bind group each frame from the current [`ViewUniforms`].
fn prepare_trail_view_bind_group(
    mut bind_group: ResMut<TrailViewBindGroup>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<TrailPipeline>,
    view_uniforms: Res<ViewUniforms>,
) {
    let Some(view_binding) = view_uniforms.uniforms.binding() else {
        return;
    };

    let view_layout = pipeline_cache.get_bind_group_layout(&pipeline.view_layout);
    bind_group.0 = Some(render_device.create_bind_group(
        "trail_view_bind_group",
        &view_layout,
        &BindGroupEntries::single(view_binding),
    ));
}

/// Scratch accumulators for building one batch, reused across frames.
#[derive(Default)]
struct BatchScratch {
    headers: Vec<TrailHeader>,
    points: Vec<TrailPoint>,
    styles: Vec<TrailStyle>,
    bytes: Vec<u8>,
}

/// Packs every trail into per-mode shared storage buffers (headers, ring points,
/// styles) and uploads them, ready for one instanced draw per mode.
#[allow(clippy::too_many_arguments)]
fn prepare_trail_batches(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline: Res<TrailPipeline>,
    pipeline_cache: Res<PipelineCache>,
    changed: Res<TrailsChanged>,
    mut batches: ResMut<TrailBatches>,
    trails: Query<(&TrailData, &TrailStyle, &TrailRenderMode)>,
    mut scratch: Local<[BatchScratch; 3]>,
) {
    // Nothing changed → last frame's buffers and bind groups are still valid.
    // (The first frame reports changed, so they build then.)
    if !changed.0 {
        return;
    }

    for s in scratch.iter_mut() {
        s.headers.clear();
        s.points.clear();
        s.styles.clear();
    }

    let mut max_verts = [0u32; 3];

    for (trail, style, mode) in &trails {
        let m = *mode as usize;
        let s = &mut scratch[m];

        // The shader walks `capacity` points from `offset`, so exactly that many
        // must be uploaded. Catch a hand-built instance that broke the invariant.
        debug_assert_eq!(
            trail.cpu_data.len(),
            trail.header.capacity as usize,
            "TrailData invariant violated: cpu_data.len() must equal header.capacity"
        );

        let mut header = trail.header.clone();
        header.offset = s.points.len() as u32;
        let length = header.length;
        s.headers.push(header);
        s.styles.push(style.clone());
        s.points.extend_from_slice(&trail.cpu_data);

        // Size the draw to live points (2 verts each), not full capacity.
        max_verts[m] = max_verts[m].max(length * 2);
    }

    let layout = pipeline_cache.get_bind_group_layout(&pipeline.batch_layout);

    for m in 0..3 {
        let s = &mut scratch[m];
        let batch = &mut batches.modes[m];

        batch.instance_count = s.headers.len() as u32;
        batch.max_verts = max_verts[m];

        if s.headers.is_empty() {
            batch.bind_group = None;
            continue;
        }

        let h = ensure_and_write(
            &render_device,
            &render_queue,
            &mut batch.headers,
            &s.headers,
            &mut s.bytes,
            "trail_batch_headers",
        );
        let p = ensure_and_write(
            &render_device,
            &render_queue,
            &mut batch.points,
            &s.points,
            &mut s.bytes,
            "trail_batch_points",
        );
        let st = ensure_and_write(
            &render_device,
            &render_queue,
            &mut batch.styles,
            &s.styles,
            &mut s.bytes,
            "trail_batch_styles",
        );

        // Only rebuild the bind group when a buffer was (re)allocated.
        if h || p || st || batch.bind_group.is_none() {
            batch.bind_group = Some(render_device.create_bind_group(
                "trail_batch_bind_group",
                &layout,
                &BindGroupEntries::sequential((
                    batch.headers.buffer.as_ref().unwrap().as_entire_binding(),
                    batch.points.buffer.as_ref().unwrap().as_entire_binding(),
                    batch.styles.buffer.as_ref().unwrap().as_entire_binding(),
                )),
            ));
        }
    }
}

/// Encodes `data` (a `ShaderType` array) into `scratch`, ensures `buffer` is at
/// least that large (reallocating with growth headroom if not), and queues the
/// upload. Returns `true` if the buffer was reallocated (so the caller knows to
/// rebuild the bind group).
fn ensure_and_write<T>(
    render_device: &RenderDevice,
    render_queue: &RenderQueue,
    target: &mut GrowBuffer,
    data: &[T],
    scratch: &mut Vec<u8>,
    label: &'static str,
) -> bool
where
    T: bevy::render::render_resource::ShaderType
        + bevy::render::render_resource::encase::ShaderSize
        + bevy::render::render_resource::encase::internal::WriteInto,
{
    scratch.clear();
    {
        let mut wrapper = EncaseStorageBuffer::new(&mut *scratch);
        wrapper.write(data).unwrap();
    }
    let size = scratch.len() as u64;

    let mut reallocated = false;
    if target.buffer.is_none() || target.capacity < size {
        // Grow with 50% headroom so a steadily increasing trail count doesn't
        // reallocate every frame.
        let new_cap = (size + size / 2).max(size);
        target.buffer = Some(render_device.create_buffer(&BufferDescriptor {
            label: Some(label),
            size: new_cap,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        target.capacity = new_cap;
        reallocated = true;
    }

    render_queue.write_buffer(target.buffer.as_ref().unwrap(), 0, scratch);
    reallocated
}

/// The custom draw commands Bevy executes for each batch phase item.
type DrawTrailCommands = (SetItemPipeline, SetTrailViewBindGroup<0>, DrawTrailBatch);

pub struct TrailRenderPlugin;

impl Plugin for TrailRenderPlugin {
    fn build(&self, app: &mut App) {
        // Main World
        app.add_plugins((
            ExtractComponentPlugin::<TrailData>::default(),
            ExtractComponentPlugin::<TrailStyle>::default(),
            ExtractComponentPlugin::<TrailRenderMode>::default(),
            ExtractComponentPlugin::<TrailBatchAnchor>::default(),
            ExtractResourcePlugin::<TrailsChanged>::default(),
        ))
        // Runs after the Update systems that mutate trails (emit/sync) and
        // before extract, so the flag reflects the same data that gets extracted.
        .init_resource::<TrailsChanged>()
        .add_systems(Last, detect_trail_changes);

        // One anchor entity per blend mode (see `TrailBatchAnchor`).
        for mode in MODES {
            app.world_mut().spawn(TrailBatchAnchor { mode });
        }

        // Render World
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_render_command::<Opaque3d, DrawTrailCommands>()
            .init_resource::<TrailViewBindGroup>()
            .init_resource::<TrailBatches>()
            .add_systems(
                Render,
                (
                    prepare_trail_batches.in_set(RenderSystems::PrepareResources),
                    prepare_trail_view_bind_group.in_set(RenderSystems::PrepareBindGroups),
                    queue_custom_phase_item.in_set(RenderSystems::Queue),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        // TrailPipeline needs RenderDevice, which isn't created until App::run.
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<TrailPipeline>();
    }
}

/// Enqueues one batched phase item per (view, non-empty blend mode). Each item
/// expands into a single instanced draw covering all trails of that mode.
#[allow(clippy::too_many_arguments)]
fn queue_custom_phase_item(
    pipeline_cache: Res<PipelineCache>,
    mut pipeline: ResMut<TrailPipeline>,
    mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    batches: Res<TrailBatches>,
    anchors: Query<(Entity, &MainEntity, &TrailBatchAnchor)>,
    views: Query<(&ExtractedView, &Msaa)>,
    mut next_tick: Local<Tick>,
) {
    let draw_custom_phase_item = opaque_draw_functions.read().id::<DrawTrailCommands>();

    // The renderer-owned anchor entity that represents each blend mode's batch.
    let mut anchor_by_mode: [Option<(Entity, MainEntity)>; 3] = [None; 3];
    for (entity, main, anchor) in anchors.iter() {
        anchor_by_mode[anchor.mode as usize] = Some((entity, *main));
    }

    for (view, msaa) in views.iter() {
        let Some(opaque_phase) = opaque_render_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        for m in 0..3 {
            let batch = &batches.modes[m];
            if batch.instance_count == 0 {
                continue;
            }
            let Some(representative) = anchor_by_mode[m] else {
                continue;
            };

            let mode = MODES[m];
            let Ok(pipeline_id) = pipeline.variants.specialize(
                &pipeline_cache,
                TrailPipelineKey {
                    msaa: *msaa,
                    mode,
                    hdr: view.hdr,
                },
            ) else {
                continue;
            };

            // Bump the tick so Bevy rebuilds this item's bin. Only three items
            // exist, so doing it every frame is free; the `Tick` wraps safely.
            let this_tick = next_tick.get() + 1;
            next_tick.set(this_tick);

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
                representative,
                InputUniformIndex::default(),
                BinnedRenderPhaseType::NonMesh,
                *next_tick,
            );
        }
    }
}

#[derive(Resource)]
struct TrailPipeline {
    /// Specialized pipeline variants; also keeps the shader handle alive via the
    /// base descriptor.
    variants: Variants<RenderPipeline, TrailPipelineSpecializer>,
    /// Layout for the view uniform bind group (group 0).
    view_layout: BindGroupLayoutDescriptor,
    /// Layout for the batched trail buffers bind group (group 1).
    batch_layout: BindGroupLayoutDescriptor,
}

impl FromWorld for TrailPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let shader = asset_server.load("shaders/trail_drawing.wgsl");

        let view_layout = BindGroupLayoutDescriptor::new(
            "trail_view_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX,
                uniform_buffer::<ViewUniform>(true),
            ),
        );

        // group 1: headers, points, styles — all read-only storage arrays.
        let batch_layout = BindGroupLayoutDescriptor::new(
            "trail_batch_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    storage_buffer_read_only_sized(false, None),
                    storage_buffer_read_only_sized(false, None),
                    storage_buffer_read_only_sized(false, None),
                ),
            ),
        );

        let base_descriptor = RenderPipelineDescriptor {
            label: Some("trail render pipeline".into()),
            layout: vec![view_layout.clone(), batch_layout.clone()],
            vertex: VertexState {
                shader: shader.clone(),
                // No vertex buffers; geometry is generated in the shader.
                ..default()
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                cull_mode: None,
                ..default()
            },
            fragment: Some(FragmentState {
                shader: shader.clone(),
                targets: vec![Some(ColorTargetState {
                    // Placeholder format; the specializer overwrites this with
                    // the view's actual format (HDR vs. default) based on
                    // `TrailPipelineKey::hdr`.
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

        Self {
            variants,
            view_layout,
            batch_layout,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, SpecializerKey)]
struct TrailPipelineKey {
    msaa: Msaa,
    mode: TrailRenderMode,
    /// Whether the target view is HDR, so the color target format matches it.
    hdr: bool,
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

        // Pick the blend state for this trail's render mode. The fragment shader
        // outputs straight (non-premultiplied) color, so additive scales the
        // contribution by alpha and transparent uses standard alpha blending.
        let blend = match key.mode {
            TrailRenderMode::Opaque => None,
            TrailRenderMode::Additive => Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            }),
            TrailRenderMode::Transparent => Some(BlendState::ALPHA_BLENDING),
        };

        // Match the view's render target format so HDR cameras (e.g. for bloom)
        // don't hit a pipeline/target format mismatch.
        let format = if key.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut().flatten() {
                target.format = format;
                target.blend = blend;
            }
        }

        Ok(key)
    }
}
