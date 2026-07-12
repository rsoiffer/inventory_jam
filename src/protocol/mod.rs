pub mod channels;
pub mod inputs;
pub mod messages;
pub mod player;

use bevy::prelude::*;

pub use channels::ServerEventsChannel;
pub use inputs::PlayerAction;
pub use messages::DebugServerMessage;
pub use player::{PlayerAimDirection, PlayerColor, PlayerId, PlayerPosition};

pub use crate::projectile::protocol::PlayerProjectile;
pub use crate::projectile::shared::{ProjectileLifetime, ProjectilePosition};

#[derive(Clone)]
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        channels::register(app);
        messages::register(app);
        inputs::register(app);
        player::register(app);
        crate::projectile::protocol::register(app);
        crate::enemy::protocol::register(app);
    }
}
