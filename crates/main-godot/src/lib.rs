use bevy::{MinimalPlugins, app::App, state::app::StatesPlugin};
use godot_bevy::{plugins::input::GodotActionsPlugin, prelude::bevy_app};

#[bevy_app]
fn build_app(app: &mut App) {
    // This example uses most godot-bevy features
    app.add_plugins(MinimalPlugins)
        .add_plugins(GodotActionsPlugin)
        .add_plugins(StatesPlugin);
}
