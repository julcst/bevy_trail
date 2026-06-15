//! Automatic emission: trails that follow their entity's [`GlobalTransform`].

use std::sync::Arc;

use bevy::prelude::*;

use crate::types::{Trail, TrailData, TrailHeader, TrailPoint};

/// Makes a trail trace its entity's path automatically.
///
/// Add it to any entity with a [`Transform`]; it pulls in [`Trail`] via
/// `#[require]`, so it's the only component you need for the common case:
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
    /// If false, the head point is updated every frame even when the entity
    /// hasn't moved far enough to emit a new one.
    pub lazy: bool,
    /// Minimum travel distance before a new point is emitted. `None` derives a
    /// spacing that fills the ring over one
    /// [`Trail::max_length`](crate::types::Trail) of travel.
    pub min_distance: Option<f32>,
}

impl TrailEmitter {
    /// Minimum travel distance between points: explicit
    /// [`min_distance`](Self::min_distance), else `max_length / capacity`.
    /// `None` (no length budget and no explicit distance) means emit on any
    /// movement — see [`emit_points_system`].
    fn spacing(&self, header: &TrailHeader) -> Option<f32> {
        self.min_distance.or_else(|| {
            let derived = header.max_length / header.capacity.max(1) as f32;
            (derived > 0.0).then_some(derived)
        })
    }
}

/// Samples each emitter's [`GlobalTransform`] into its ring (in parallel),
/// gated by spacing, then clips the tail by `max_length`/`max_time`.
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

            let should_emit = match (&emitter.last, emitter.spacing(&trail.header)) {
                // First point always emits.
                (None, _) => true,
                // A distance budget gates emission by how far we've travelled.
                (Some(last), Some(spacing)) => (position - last.position).length() >= spacing,
                // No budget to derive spacing from → emit whenever we moved at all.
                (Some(last), None) => position != last.position,
            };

            if should_emit {
                // Advance the head and write the new point. `make_mut` clones the
                // ring only when it's still shared with the render world, and the
                // clone keeps the `len == capacity` invariant (no resize).
                let capacity = trail.header.capacity;
                trail.header.head = (trail.header.head + 1) % capacity;
                trail.header.length = (trail.header.length + 1).min(capacity);

                let head = trail.header.head as usize;
                Arc::make_mut(&mut trail.cpu_data)[head] = point.clone();
                emitter.last = Some(point);
            } else if !emitter.lazy {
                // Didn't emit, but keep the head pinned to the current position.
                let head = trail.header.head as usize;
                Arc::make_mut(&mut trail.cpu_data)[head] = point;
            }

            // Clip the tail. A 0 budget disables that axis; with both disabled
            // the trail is bounded only by capacity, so skip clipping.
            let clip = trail.header.max_length > 0.0 || trail.header.max_time > 0.0;
            while clip && trail.header.length > 1 {
                // Test the point one in from the tail for smoother clipping.
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
