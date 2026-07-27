/*
    Allows the protocol to register events.
*/

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::input::leafwing::prelude::{
    InputPlugin as LightyearLeafwingInputPlugin, LeafwingSequence,
};
use lightyear::input::plugin::InputPlugin as LightyearInputMessagePlugin;
use serde::{Deserialize, Serialize};

#[derive(Actionlike, Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone, Copy, Reflect)]
pub enum PlayerAction {
    #[actionlike(DualAxis)]
    Move,

    #[actionlike(DualAxis)]
    Aim,

    Fire,
    Interact,
    UseSkill,
    DebugBoostGlobalAristeia,
}

impl PlayerAction {
    pub fn default_input_map() -> InputMap<Self> {
        let mut input_map = InputMap::default();

        input_map.insert_dual_axis(Self::Move, VirtualDPad::wasd());
        input_map.insert_dual_axis(Self::Move, VirtualDPad::arrow_keys());

        input_map.insert(Self::Fire, KeyCode::Space);
        input_map.insert(Self::Fire, MouseButton::Left);

        input_map.insert(Self::Interact, KeyCode::KeyE);

        #[cfg(feature = "dev")]
        {
            input_map.insert(Self::UseSkill, KeyCode::ShiftLeft);
            input_map.insert(Self::UseSkill, KeyCode::ShiftRight);
            input_map.insert(Self::DebugBoostGlobalAristeia, KeyCode::KeyG);
        }

        input_map
    }
}

pub fn register(app: &mut App) {
    // Register InputMessage during ProtocolPlugin::build on every peer.
    // Leafwing only adds this via ServerInputPlugin (needs `server`) or
    // ClientInputPlugin (finish-time), so client-only builds otherwise get a
    // different message protocol than the dedicated server.
    app.add_plugins(LightyearInputMessagePlugin::<LeafwingSequence<PlayerAction>>::default());
    app.add_plugins(LightyearLeafwingInputPlugin::<PlayerAction>::default());
}
