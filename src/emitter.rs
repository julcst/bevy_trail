use bevy::ecs::component::Component;


#[derive(Component)]
pub struct TrailEmitter {
    pub max_points: usize,
    pub max_length: Option<f32>,
    pub max_time: Option<f32>,
    pub distance_threshold: f32,
}
