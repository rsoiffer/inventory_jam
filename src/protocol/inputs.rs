/*
    Allows the protocol to register events.
*/

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::input::leafwing::prelude::InputPlugin as LightyearLeafwingInputPlugin;
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
        }

        input_map
    }
}

pub fn register(app: &mut App) {
    app.add_plugins(LightyearLeafwingInputPlugin::<PlayerAction>::default());
}
