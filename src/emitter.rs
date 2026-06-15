//! Automatic emission: trails that follow their entity's [`GlobalTransform`].

use std::sync::Arc;

use bevy::prelude::*;

use crate::types::{Trail, TrailData, TrailHeader, TrailPoint};

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
    /// Minimum world-space distance the entity must travel before a new point
    /// is emitted. `None` derives a spacing that fills the ring buffer over one
    /// [`Trail::max_length`](crate::types::Trail) of travel.
    pub min_distance: Option<f32>,
}

impl TrailEmitter {
    /// Resolves the minimum travel distance between emitted points: an explicit
    /// [`min_distance`](Self::min_distance), else `max_length / capacity` so a
    /// default trail keeps roughly `capacity` points across its visible length.
    ///
    /// Returns `None` when there is no length budget to derive from
    /// (`max_length == 0`, length clipping disabled) and no explicit
    /// `min_distance`; the emitter then emits on any movement (see
    /// [`emit_points_system`]). Set `min_distance` to gate spacing in that case.
    fn spacing(&self, header: &TrailHeader) -> Option<f32> {
        self.min_distance.or_else(|| {
            let derived = header.max_length / header.capacity.max(1) as f32;
            (derived > 0.0).then_some(derived)
        })
    }
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

            let should_emit = match (&emitter.last, emitter.spacing(&trail.header)) {
                // First point always emits.
                (None, _) => true,
                // A distance budget gates emission by how far we've travelled.
                (Some(last), Some(spacing)) => (position - last.position).length() >= spacing,
                // No budget to derive spacing from → emit whenever we moved at all.
                (Some(last), None) => position != last.position,
            };

            if should_emit {
                // Increment header
                let capacity = trail.header.capacity;
                trail.header.head = (trail.header.head + 1) % capacity;
                trail.header.length = (trail.header.length + 1).min(capacity);

                // Write new head. `make_mut` only clones the ring when it is still
                // shared with the render world (i.e. when this trail changed); the
                // clone preserves the `len == capacity` invariant, so no resize.
                let head = trail.header.head as usize;
                Arc::make_mut(&mut trail.cpu_data)[head] = point.clone();
                emitter.last = Some(point);
            } else if !emitter.lazy {
                // Overwrite head
                let head = trail.header.head as usize;
                Arc::make_mut(&mut trail.cpu_data)[head] = point;
            }

            // Clip the tail. A 0 budget disables that axis; with both disabled
            // the trail is bounded only by capacity, so skip clipping entirely.
            let clip = trail.header.max_length > 0.0 || trail.header.max_time > 0.0;
            while clip && trail.header.length > 1 {
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
