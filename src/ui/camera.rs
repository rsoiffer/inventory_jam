use crate::settings::GameSettings;
use bevy::camera::ClearColorConfig;
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

#[cfg(feature = "server")]
use crate::app::ServerState;

#[derive(Clone)]
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init);
        app.add_systems(Update, sync_camera_scale.run_if(not_hosting));
    }
}

fn init(mut commands: Commands, settings: Res<GameSettings>) {
    let post = &settings.post_processing;
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(post.clear_color),
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scale: settings.camera.scale,
            ..OrthographicProjection::default_2d()
        }),
        Tonemapping::TonyMcMapface,
        Bloom {
            intensity: post.bloom_intensity,
            low_frequency_boost: post.bloom_low_frequency_boost,
            ..Bloom::OLD_SCHOOL
        },
        DebandDither::Enabled,
    ));
}

/// Keeps the visible world diagonal constant across resolutions, matching the
/// default 1024x768 window at `camera.scale`.
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
    orthographic.scale = settings.camera.scale * REFERENCE_DIAGONAL / diagonal;
}

#[cfg(feature = "server")]
fn not_hosting(server_state: Res<State<ServerState>>) -> bool {
    *server_state.get() != ServerState::Hosting
}

#[cfg(not(feature = "server"))]
fn not_hosting() -> bool {
    true
}
