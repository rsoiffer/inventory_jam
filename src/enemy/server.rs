use bevy::prelude::*;
use lightyear::prelude::*;
use rand::Rng;

use crate::enemy::protocol::EnemyPosition;

const ENEMY_SPAWN_RADIUS: f32 = 150.0;

pub(crate) fn spawn_enemy(input: Option<Res<ButtonInput<KeyCode>>>, mut commands: Commands) {
    if input.is_some_and(|input| input.just_pressed(KeyCode::Space)) {
        let mut rng = rand::rng();
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let radius = rng
            .random_range(0.0..=ENEMY_SPAWN_RADIUS * ENEMY_SPAWN_RADIUS)
            .sqrt();
        let position = Vec2::new(angle.cos(), angle.sin()) * radius;

        let entity = commands
            .spawn((
                EnemyPosition(position),
                Replicate::to_clients(NetworkTarget::All),
                InterpolationTarget::to_clients(NetworkTarget::All),
                Name::new("Enemy"),
            ))
            .id();

        info!("Create enemy entity {:?} at {:?}", entity, position);
    }
}
