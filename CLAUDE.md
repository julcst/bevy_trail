# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`bevy_trail` is a GPU-accelerated trail (ribbon) rendering plugin for **Bevy 0.18**. It renders camera-facing trail billboards by packing every trail into shared GPU storage buffers and issuing **one instanced draw call per blend mode**, expanding geometry in the vertex shader (no vertex/index buffers).

## Commands

```bash
cargo build                                    # build the library
cargo clippy --all-targets                     # lint (CI-equivalent check)

# Examples (each is a runnable app; --release strongly recommended for visuals/perf)
cargo run --release --example render_test      # main visual demo
cargo run --release --example double_pendulum  # chaotic-motion demo
cargo run --release --example emitter_test     # emitter spacing behavior
cargo run --release --example screenshot_test  # renders trails, saves /tmp/trail_shot.png, exits
cargo run --release --example stress_bench -- 100   # spawn N trails, print frame-time stats, exit
cargo run --release --example bufferless_drawing    # reference: raw Bevy custom-render example (not part of the crate API)
```

There are **no unit tests**. Verification is visual/behavioral via the examples — `screenshot_test` (correctness) and `stress_bench` (performance) are designed to be driven non-interactively from a script.

## Architecture

The crate is deliberately split so a trail is **plain data, not a render object** — a `Trail` can be attached to any entity (even one with its own `Mesh3d`) without interfering with that entity's rendering, because trails carry no `Visibility`/`Aabb`/visibility-class and add no per-entity draw or bind group.

Data flows main world → render world each frame:

- **`types.rs`** — user-facing components and GPU layout structs.
  - `Trail` (config: `capacity`, `max_length`, `max_time`) `#[require]`s `TrailStyle` + `TrailRenderMode`.
  - `TrailData` is the renderable ring-buffer state, inserted/maintained by the plugin from `Trail`. **Invariant: `cpu_data.len() == header.capacity`**; build via `TrailData::new` / `from_points`, never the struct literal. `cpu_data` is `Arc<Vec<TrailPoint>>` so extraction is a cheap pointer clone and the emitter only deep-copies (via `Arc::make_mut`) when a trail actually changes.
  - Structs carrying GPU data derive `ShaderType` and **must stay field-for-field in sync with the matching WGSL structs** in `assets/shaders/trail_drawing.wgsl` (`Header`, `TrailPoint`, `TrailStyle`).
- **`emitter.rs`** — `TrailEmitter` `#[require]`s `Trail`; `emit_points_system` samples `GlobalTransform` into the ring (parallel via `par_iter_mut`), gated by `min_distance` spacing, and clips the tail by `max_length`/`max_time` (a `0.0` budget disables that axis).
- **`lib.rs`** — `TrailPlugin` wires it together. `Update` ordering is `TrailSystems::Init` (insert `TrailData` for new trails) → `TrailSystems::Emit`.
- **`render.rs`** — `TrailRenderPlugin`, the bulk of the complexity. See below.

### Rendering pipeline (`render.rs`)

- Trails are extracted via `ExtractComponent` and packed per blend mode into three shared storage buffers — `headers`, `points` (all rings concatenated; each trail starts at `header.offset`), `styles` — then drawn with one instanced draw call where **instance N renders trail N**.
- **Three blend modes** (`Opaque`/`Additive`/`Transparent`) → one `GpuBatch` each, indexed by `TrailRenderMode as usize`. The `MODES` const array fixes that ordering; a mode round-trips through `mode as usize`.
- **`GrowBuffer`** keeps GPU buffers across frames and only grows them (with 50% headroom), avoiding per-frame reallocation. Bind groups are rebuilt only when a buffer is reallocated.
- **`TrailsChanged`** (computed in `Last` via `detect_trail_changes`, then extracted) gates the whole repack: a fully static scene of pre-baked trails costs ~nothing. Change detection is O(changes), not O(trails).
- **`TrailBatchAnchor`**: three renderer-owned, mesh-less entities (one per mode) serve as the representative entity for each batch's binned phase item. This avoids a `MainEntity` cache collision that would occur if a real trail entity (which may render its own mesh) were reused — that would make the trail item and mesh item fight over one phase-cache slot and drop both.
- The pipeline is **specialized** (`TrailPipelineSpecializer` / `TrailPipelineKey`) over MSAA, blend mode, and HDR — the color target format is set to match the view (HDR vs default) so HDR cameras (bloom) don't hit a format mismatch.
- Items are enqueued into the `Opaque3d` phase; `queue_custom_phase_item` bumps a local `Tick` every frame to force the binned phase to rebuild the (only three) items.

### Shaders (`assets/shaders/`)

- `trail_drawing.wgsl` is the live shader used by the plugin. It reads the three storage buffers, walks each trail's ring via `offset`/`head`/`capacity` modulo arithmetic, computes a tangent, and expands each point into 2 vertices of a camera-facing `TriangleStrip` billboard. WGSL structs mirror the Rust `ShaderType` structs — **edit both together**.
- `bufferless_drawing.wgsl` belongs to the standalone `bufferless_drawing` example (a Bevy custom-render reference), not the crate.

## Conventions

- Targets Bevy **0.18.1** specifically; rendering internals (render phases, specialization, `BindGroupLayoutDescriptor`) track Bevy's fast-moving render API, so check the installed Bevy version before porting patterns from older/newer Bevy docs.
- When changing any GPU-facing struct, update the Rust struct, its WGSL counterpart, and any `debug_assert`/invariant that depends on its layout together.
