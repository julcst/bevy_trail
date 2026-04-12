use bevy::prelude::*;

use crate::types::{TrailData, TrailPoint};

#[derive(Component)]
pub struct TrailEmitter {
    pub max_points: usize,
    pub max_length: Option<f32>,
    pub max_time: Option<f32>,
    pub distance_threshold: f32,
    last: Option<TrailPoint>,
}

impl Default for TrailEmitter {
    fn default() -> Self {
        Self {
            max_points: 128,
            max_length: None,
            max_time: None,
            distance_threshold: 0.0,
            last: None,
        }
    }
}

fn add_point(trail: &mut TrailData, point: TrailPoint) {
    trail
        .cpu_data
        .resize_with(trail.header.capacity as usize, Default::default);
    trail.cpu_data[trail.header.head as usize] = point;
    trail.header.head = (trail.header.head + 1) % trail.header.capacity;
}

impl TrailEmitter {
    fn emit_point(&mut self, trail: &mut TrailData, point: TrailPoint) {
        if self.last.as_ref().is_some_and(|last| {
            (point.position - last.position).length_squared()
                > self.distance_threshold * self.distance_threshold
        }) {
            add_point(trail, point.clone());
            self.last = Some(point);
        }
    }
}

fn emit_points_system(mut trails: Query<(&GlobalTransform, &mut TrailData, &mut TrailEmitter)>) {
    trails
        .par_iter_mut()
        .for_each(|(transform, mut trail, mut emitter)| {
            let position = transform.translation();
            let velocity = emitter
                .last
                .as_ref()
                .map_or(Vec3::ZERO, |last| position - last.position);
            let point = TrailPoint {
                position,
                width: 0.1,
                color: Vec4::ONE,
                velocity,
                t: 0.0,
            };
            emitter.emit_point(&mut trail, point);
        });
}
