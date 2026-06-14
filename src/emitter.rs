//! Automatic emission: trails that follow their entity's [`GlobalTransform`].

use bevy::prelude::*;

use crate::types::{Trail, TrailData, TrailPoint};

/// Makes a trail trace the path of its entity automatically.
///
/// Add it to any entity with a [`Transform`]; it pulls in [`Trail`] (and thus
/// the whole trail setup) via `#[require]`, so this is the only component you
/// need for the common case:
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_trail::prelude::*;
/// # fn setup(mut commands: Commands) {
/// commands.spawn((Transform::default(), TrailEmitter::default()));
/// # }
/// ```
#[derive(Component, Default)]
#[require(Trail)]
pub struct TrailEmitter {
    pub last: Option<TrailPoint>,
    /// If false, the emitter updates the head point every frame even when it
    /// hasn't moved far enough to emit a new point.
    pub lazy: bool,
}

pub(crate) fn emit_points_system(
    time: Res<Time>,
    mut trails: Query<(&GlobalTransform, &mut TrailData, &mut TrailEmitter)>,
) {
    trails
        .par_iter_mut()
        .for_each(|(transform, mut trail, mut emitter)| {
            let position = transform.translation();

            let time = time.elapsed_secs();
            let length = emitter.last.as_ref().map_or(0.0, |last| {
                last.length + (position - last.position).length()
            });

            let point = TrailPoint {
                position,
                time,
                custom: Vec3::ZERO,
                length,
            };

            trail.header.current_time = point.time;
            trail.header.current_length = point.length;

            let should_emit = emitter.last.as_ref().is_none_or(|last| {
                let threshold = trail.header.max_length / trail.header.capacity as f32;
                (position - last.position).length() >= threshold
            });

            if should_emit {
                // Increment header
                let capacity = trail.header.capacity as usize;
                trail.cpu_data.resize_with(capacity, Default::default);
                trail.header.head = (trail.header.head + 1) % trail.header.capacity;
                trail.header.length = (trail.header.length + 1).min(trail.header.capacity);

                // Write new head
                let head = trail.header.head as usize;
                trail.cpu_data[head] = point.clone();
                emitter.last = Some(point);
            } else if !emitter.lazy {
                // Overwrite head
                let head = trail.header.head as usize;
                trail.cpu_data[head] = point;
            }

            // Clip the trail length
            while trail.header.length > 1 {
                // We take the point before the end (+1) for smoother tail clipping
                let end = (trail.header.head + trail.header.capacity - trail.header.length + 1)
                    % trail.header.capacity;
                let point = &trail.cpu_data[end as usize];
                if point.length >= trail.header.current_length - trail.header.max_length
                    || point.time >= trail.header.current_time - trail.header.max_time
                {
                    break;
                }
                trail.header.length -= 1;
            }
        });
}
