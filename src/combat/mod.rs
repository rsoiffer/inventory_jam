mod protocol;

#[cfg(feature = "gui")]
mod popup;

use bevy::app::{App, Plugin};

pub use protocol::HitFlash;

#[cfg(feature = "gui")]
pub use popup::{DamageHitId, DamagePopupKind, SpawnDamagePopup};

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(protocol::CombatProtocolPlugin);

        #[cfg(feature = "gui")]
        app.add_plugins(popup::DamagePopupPlugin);
    }
}
