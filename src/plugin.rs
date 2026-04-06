//! Main plugin for trail rendering system.

use crate::components::{Trail, TrailEmitter, TrailEmitterConfig};
use crate::render::{TrailMaterial, TrailRenderPlugin, TrailUniforms};
use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;

/// Main plugin for the trail rendering system.
///
/// Add this to your Bevy app to enable trail rendering:
///
/// ```ignore
/// app.add_plugins(TrailPlugin::default());
/// ```
#[derive(Default)]
pub struct TrailPlugin;

impl Plugin for TrailPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TrailRenderPlugin);
        app.add_systems(
            FixedUpdate,
            (spawn_emitter_trails, update_emitter_trails).chain(),
        );
    }
}

/// System that spawns `Trail` components for entities with `TrailEmitter`.
fn spawn_emitter_trails(
    mut commands: Commands,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut materials: ResMut<Assets<TrailMaterial>>,
    query: Query<(Entity, &TrailEmitterConfig), Added<TrailEmitter>>,
) {
    for (entity, config) in query.iter() {
        assert!(
            config.sampling.max_points >= 2,
            "TrailSamplingConfig.max_points must be >= 2"
        );
        let capacity = config.sampling.max_points as usize;
        let gpu_buffer = buffers.add(ShaderStorageBuffer::from(vec![[0.0; 4]; capacity * 3]));

        let material = materials.add(TrailMaterial {
            uniforms: TrailUniforms::for_config(config.sampling.max_points, &config.metadata),
            points: gpu_buffer.clone(),
        });

        let trail = Trail::new_with_gpu(
            &config.sampling,
            config.metadata.clone(),
            gpu_buffer,
            material,
        );
        commands.entity(entity).insert(trail);
    }
}

/// System that updates trails by sampling positions from `GlobalTransform`.
fn update_emitter_trails(
    mut query: Query<(&GlobalTransform, &TrailEmitterConfig, &mut Trail), With<TrailEmitter>>,
    time: Res<Time>,
) {
    let delta = time.delta_secs();

    for (transform, config, mut trail) in query.iter_mut() {
        let position = transform.translation();
        let velocity = if delta > 0.0 {
            (position - trail.last_sample_position) / delta
        } else {
            Vec3::ZERO
        };

        if trail.try_sample(position, velocity, &config.sampling, delta) {
            if let Some(color_fn) = &config.color_fn {
                let newest = (trail.head + trail.capacity - 1) % trail.capacity;
                let point = &mut trail.points[newest as usize];
                point.color = color_fn(point.position, point.velocity);
            }
        }

        trail.update(&config.sampling, delta);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let _plugin = TrailPlugin;
    }
}
