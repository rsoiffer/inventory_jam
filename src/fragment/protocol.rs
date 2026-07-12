use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

/// Authoritative fragment data replicated from the server.
///
/// `collector` becomes `Some` as soon as the server awards the fragment.
/// Clients then animate their local visual toward that player until the server
/// despawns the fragment.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Fragment {
    pub origin: Vec2,
    pub collector: Option<PeerId>,
}

#[cfg(feature = "server")]
impl Fragment {
    pub const fn available(origin: Vec2) -> Self {
        Self {
            origin,
            collector: None,
        }
    }
}

/// The number of volatile fragments currently carried by a player.
///
/// This belongs to the Fragment feature even though it is attached to a player
/// entity. Extraction and death can later mutate or clear this component
/// through the feature's public API.
#[derive(
    Component, Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, Deref, DerefMut,
)]
pub struct CarriedFragments(pub u32);

pub fn register(app: &mut App) {
    app.component::<Fragment>().replicate();
    app.component::<CarriedFragments>().replicate();
}
