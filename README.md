# bevy_trail

Production-oriented GPU trail rendering for Bevy.

`bevy_trail` is an emitter-driven trail system designed for dynamic gameplay effects. It uses fixed-capacity ring buffers, storage-buffer uploads, and shader-side ribbon expansion to keep CPU overhead predictable while supporting high trail update rates.

## Why This Crate

- Stream-friendly GPU data path for frequently changing trails
- Clear ECS integration with emitter/config components
- Shader-side orientation and procedural styling hooks
- Practical baseline for extending toward segment joins/caps and advanced transparency

## Feature Checklist

### Implemented

- [x] `TrailPlugin` integration for setup and runtime systems
- [x] Emitter-based workflow with `TrailEmitter` + `TrailEmitterConfig`
- [x] Fixed-capacity ring storage (`head`, `len`, `capacity`) per trail
- [x] Sampling controls: minimum dt, maximum dt, distance threshold
- [x] Lifetime-based culling of stale points
- [x] Per-point payload: position, velocity, width, color, spawn time, cumulative length
- [x] GPU storage-buffer upload path for trail points
- [x] Capacity-keyed strip mesh cache
- [x] Shader-side ribbon expansion from ring-buffer data
- [x] Billboard orientation from view direction and motion, with tangent fallbacks
- [x] Right-vector continuity stabilization to reduce curve frame flips
- [x] Width taper controls (`base_width`, `taper_factor`)
- [x] Metadata-driven alpha compositing API (`AlphaMode`)
- [x] Material blend selection from metadata at runtime
- [x] Visual and smoke examples
- [x] Stress example for many emitters (`trail_stress`)

### Planned / Missing

- [ ] Join styles (miter, bevel, round)
- [ ] Cap styles (butt, square, round)
- [ ] Segment-instanced rendering path (optional alternative to strip)
- [ ] Texture support with UV generation and scrolling controls
- [ ] More robust transparent self-overlap strategy
- [ ] Depth-fade intersection support against opaque geometry
- [ ] Pipeline specialization keys for transparent/depth/HDR variants
- [ ] Public antialias/falloff quality controls
- [ ] Curve-based authoring for width/color over normalized trail age or length
- [ ] Visual regression test scenes and benchmark harness

## Architecture Overview

### Runtime Flow

1. Entities with `TrailEmitter` are initialized with `Trail` storage, material, and GPU buffer.
2. In fixed update, emitter transforms are sampled into trail points.
3. Dirty trails pack point data and upload to the storage buffer.
4. A cached strip mesh is rendered using a material that reads the point ring from GPU memory.
5. Vertex shader expands strip vertices into camera-facing trail ribbon geometry.

### Core Files

- `src/plugin.rs`: plugin wiring and emitter sampling systems
- `src/components.rs`: trail data model and sampling/culling logic
- `src/render.rs`: material, mesh cache, and GPU upload pipeline
- `assets/shaders/trail/trail_common.wgsl`: ring read helpers
- `assets/shaders/trail/trail_ring.wgsl`: vertex expansion and fragment shading

## Quick Start

```rust
use bevy::prelude::*;
use bevy_trail::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TrailPlugin)
        .run();
}
```

See `examples/trail_visual.rs` for a scene setup and `examples/trail_stress.rs` for load testing.

## Running Examples

```bash
cargo run --example trail_visual
cargo run --example trail_smoke
cargo run --example trail_stress
```

## Performance Notes

- Ring-buffer storage keeps memory bounded per emitter.
- Storage-buffer uploads avoid CPU-side mesh rebuilds per point.
- The current strip path is efficient, while higher-fidelity corner handling (joins/caps) is planned as an opt-in quality tier.

## License

Dual licensed under `MIT OR Apache-2.0`.
