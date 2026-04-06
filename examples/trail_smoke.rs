use bevy::prelude::*;
use bevy_trail::components::Trail;
use bevy_trail::types::{TrailMetadata, TrailSamplingConfig};

fn main() {
    let config = TrailSamplingConfig {
        min_sample_dt: 0.001,
        distance_threshold: 0.05,
        max_sample_dt: 0.05,
        max_points: 128,
        point_lifetime_secs: 10.0,
    };

    let mut trail = Trail::new(
        &config,
        TrailMetadata {
            base_width: 0.2,
            taper_factor: 0.35,
            custom_0: Vec4::new(1.0, 0.5, 0.2, 1.0),
            custom_1: Vec4::ZERO,
        },
    );

    let mut t = 0.0_f32;
    let dt = 1.0 / 120.0;

    for _ in 0..240 {
        let angle = t * 4.0;
        let pos = Vec3::new(angle.cos() * 2.0, 0.25, angle.sin() * 2.0);
        let vel = Vec3::new(-angle.sin() * 8.0, 0.0, angle.cos() * 8.0);
        let _ = trail.try_sample(pos, vel, &config, dt);
        trail.update(&config, dt);
        t += dt;
    }

    let after_sampling = trail.point_count();
    println!("trail points after sampling: {}", after_sampling);

    assert!(
        after_sampling > 0,
        "smoke test failed: no points were sampled"
    );

    for _ in 0..240 {
        let _ = trail.try_sample(Vec3::ZERO, Vec3::ZERO, &config, dt);
        trail.update(&config, dt);
    }

    let after_cull = trail.point_count();
    println!("trail points after culling window: {}", after_cull);

    assert!(
        after_cull <= config.max_points,
        "smoke test failed: point count exceeded max_points"
    );

    println!("trail smoke example passed");
}
