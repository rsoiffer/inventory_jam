use crate::settings::GameSettings;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

#[cfg(feature = "server")]
use crate::app::ServerState;

#[derive(Clone)]
pub struct HelpTextPlugin;

impl Plugin for HelpTextPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init);
        app.add_systems(Update, sync_camera_scale.run_if(not_hosting));
    }
}

fn init(mut commands: Commands, settings: Res<GameSettings>) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: settings.player.camera_scale,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

/// Keeps the visible world diagonal constant across resolutions, matching the
/// default 1024x768 window at `player.camera_scale`.
fn sync_camera_scale(
    window: Single<&Window, With<PrimaryWindow>>,
    mut camera: Single<&mut Projection, With<Camera2d>>,
    settings: Res<GameSettings>,
) {
    let Projection::Orthographic(orthographic) = camera.as_mut() else {
        return;
    };

    const REFERENCE_DIAGONAL: f32 = 1280.0; // hypot(1024, 768)
    let diagonal = window.width().hypot(window.height()).max(1.0);
    orthographic.scale = settings.player.camera_scale * REFERENCE_DIAGONAL / diagonal;
}

#[cfg(feature = "server")]
fn not_hosting(server_state: Res<State<ServerState>>) -> bool {
    *server_state.get() != ServerState::Hosting
}

#[cfg(not(feature = "server"))]
fn not_hosting() -> bool {
    true
}
