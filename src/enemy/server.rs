use bevy::prelude::*;
use lightyear::prelude::*;

use crate::enemy::protocol::EnemyPosition;

pub(crate) fn spawn_enemy(input: Option<Res<ButtonInput<KeyCode>>>, mut commands: Commands) {
    if input.is_some_and(|input| input.just_pressed(KeyCode::Space)) {
        let entity = commands
            .spawn((
                EnemyPosition(Vec2::ZERO),
                Replicate::to_clients(NetworkTarget::All),
                InterpolationTarget::to_clients(NetworkTarget::All),
                Name::new("Enemy"),
            ))
            .id();

        info!("Create enemy entity {:?} ", entity);
    }
}
