//! Player actions replicated through Lightyear's leafwing input plugin.

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::input::leafwing::prelude::InputPlugin as LightyearLeafwingInputPlugin;
use serde::{Deserialize, Serialize};

/// Actions a player can perform, synchronized as networked input.
#[derive(Actionlike, Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone, Copy, Reflect)]
pub enum PlayerAction {
    /// Dual-axis movement input (WASD / arrow keys).
    #[actionlike(DualAxis)]
    Move,

    /// Dual-axis aim input (relative mouse movement).
    #[actionlike(DualAxis)]
    Aim,

    /// Primary fire action.
    Fire,
    /// World interaction action.
    Interact,
    /// Secondary skill action.
    UseSkill,
}

impl PlayerAction {
    /// Builds the default keyboard/mouse bindings for player actions.
    pub fn default_input_map() -> InputMap<Self> {
        let mut input_map = InputMap::default();

        // Move: WASD / Arrow Keys
        input_map.insert_dual_axis(Self::Move, VirtualDPad::wasd());
        input_map.insert_dual_axis(Self::Move, VirtualDPad::arrow_keys());

        // Aim: relative mouse movement
        input_map.insert_dual_axis(Self::Aim, MouseMove::default());

        // Fire
        input_map.insert(Self::Fire, KeyCode::Space);
        input_map.insert(Self::Fire, MouseButton::Left);

        // Interact
        input_map.insert(Self::Interact, KeyCode::KeyE);

        // Secondary skill: either Shift key
        input_map.insert(Self::UseSkill, KeyCode::ShiftLeft);
        input_map.insert(Self::UseSkill, KeyCode::ShiftRight);

        input_map
    }
}

/// Registers leafwing input replication for [`PlayerAction`].
pub fn register(app: &mut App) {
    app.add_plugins(LightyearLeafwingInputPlugin::<PlayerAction>::default());
}
