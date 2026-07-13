use crate::app::{AppState, game_is_active};
use crate::enemy::EnemyClientPlugin;
use crate::protocol::player::{PlayerColor, PlayerPosition, PlayerUsername};
use crate::shared::{ARENA_WORLD_BOUNDS, SHOP_WORLD_BOUNDS};
use bevy::prelude::*;
use bevy::sprite::Anchor;

const GRID_SPACING: f32 = 100.0;
const USERNAME_LABEL_OFFSET: Vec3 = Vec3::new(0.0, 40.0, 10.0);

#[derive(Clone)]
pub struct ExampleRendererPlugin;

impl Plugin for ExampleRendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init);
        app.add_systems(OnEnter(AppState::Playing), setup_instructions);
        app.add_systems(OnExit(AppState::Playing), cleanup_instructions);

        app.add_plugins(EnemyClientPlugin);

        app.add_systems(
            Update,
            (draw_test_worlds, draw_player_boxes, sync_username_labels).run_if(game_is_active),
        );
    }
}

fn init(mut commands: Commands) {
    commands.spawn(Camera2d);
}

pub(crate) fn draw_player_boxes(
    mut gizmos: Gizmos,
    players: Query<(&PlayerPosition, &PlayerColor)>,
) {
    for (position, color) in &players {
        gizmos.rect_2d(
            Isometry2d::from_translation(position.0),
            Vec2::ONE * 50.0,
            color.0,
        );
    }
}

fn draw_test_worlds(mut gizmos: Gizmos) {
    draw_test_world(&mut gizmos, ARENA_WORLD_BOUNDS);
    draw_test_world(&mut gizmos, SHOP_WORLD_BOUNDS);
}

/// Draws a simple top-down test arena with a grid and visible boundaries.
fn draw_test_world(gizmos: &mut Gizmos, bounds: Rect) {
    let grid_color = Color::srgba(0.35, 0.38, 0.42, 0.35);
    let axis_color = Color::srgba(0.7, 0.72, 0.75, 0.7);
    let boundary_color = Color::srgb(0.95, 0.25, 0.2);

    // Vertical grid lines
    let mut x = bounds.min.x;
    while x <= bounds.max.x {
        let color = if x.abs() < f32::EPSILON {
            axis_color
        } else {
            grid_color
        };
        gizmos.line_2d(
            Vec2::new(x, bounds.min.y),
            Vec2::new(x, bounds.max.y),
            color,
        );
        x += GRID_SPACING;
    }

    // Horizontal grid lines
    let mut y = bounds.min.y;
    while y <= bounds.max.y {
        let color = if y.abs() < f32::EPSILON {
            axis_color
        } else {
            grid_color
        };
        gizmos.line_2d(
            Vec2::new(bounds.min.x, y),
            Vec2::new(bounds.max.x, y),
            color,
        );
        y += GRID_SPACING;
    }

    // Boundary rectangle
    gizmos.rect_2d(
        Isometry2d::from_translation(bounds.center()),
        bounds.size(),
        boundary_color,
    );
}

#[derive(Component)]
struct InstructionsText;

#[derive(Component)]
struct UsernameLabel {
    player: Entity,
}

fn setup_instructions(mut commands: Commands) {
    commands.spawn((
        InstructionsText,
        Name::new("Input Control Instructions"),
        Text::new(
            "Move with WASD\nAim with Mouse\nHold Left Click to Fire\nN spawns an enemy (host/server debug)\nShift uses Skill",
        ),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn cleanup_instructions(mut commands: Commands, texts: Query<Entity, With<InstructionsText>>) {
    for entity in &texts {
        commands.entity(entity).despawn();
    }
}

fn sync_username_labels(
    mut commands: Commands,
    players: Query<(Entity, &PlayerPosition, &PlayerUsername)>,
    mut labels: Query<(Entity, &UsernameLabel, &mut Transform, &mut Text2d)>,
) {
    for (player, position, username) in &players {
        if let Some((_, _, mut transform, mut text)) = labels
            .iter_mut()
            .find(|(_, label, _, _)| label.player == player)
        {
            transform.translation = position.0.extend(0.0) + USERNAME_LABEL_OFFSET;
            if text.0 != username.0 {
                text.0.clone_from(&username.0);
            }
            continue;
        }

        commands.spawn((
            UsernameLabel { player },
            Text2d::new(username.0.clone()),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Anchor::BOTTOM_CENTER,
            Transform::from_translation(position.0.extend(0.0) + USERNAME_LABEL_OFFSET),
        ));
    }

    for (label_entity, label, _, _) in &labels {
        if players.get(label.player).is_err() {
            commands.entity(label_entity).despawn();
        }
    }
}
