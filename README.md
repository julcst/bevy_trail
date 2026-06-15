# bevy_trail

Overengineered GPU trail rendering plugin for Bevy.
Renders trails efficiently by implicitly constructing triangle strips from a ring buffer, skipping explicit vertex and element buffers.

<img width="320" height="320" alt="bevy_trail" src="https://github.com/user-attachments/assets/fc6717bd-d59e-4f13-985f-19f95ba470a1" />

## Usage

Add `TrailPlugin`, then attach a `TrailEmitter` to anything with a `Transform`.
The emitter samples the entity's path and the trail follows it automatically — a
trail is plain data, so this works even on an entity that renders its own mesh.

```rust
use bevy::prelude::*;
use bevy_trail::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TrailPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Transform::default(),
        Trail::new(64).with_max_length(2.0).with_max_time(2.0),
        TrailStyle {
            start_color: LinearRgba::WHITE,
            end_color: LinearRgba::RED,
            start_width: 0.05,
            ..default()
        },
        TrailEmitter::default(),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
```

See the [`examples/`](examples) directory for more — emitter spacing, blend
modes, pre-baked static trails, and a stress benchmark.

## TODO

- [X] Batch draw calls
- [ ] Shaded trails
- [ ] Handle jumps
