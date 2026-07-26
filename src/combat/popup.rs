use std::collections::VecDeque;

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::app::{ClientState, game_is_active};
use crate::settings::GameSettings;

const DAMAGE_POPUP_Z: f32 = 12.0;
const RECENT_HIT_CAPACITY: usize = 256;

pub struct DamagePopupPlugin;

impl Plugin for DamagePopupPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RecentDamageHits>()
            .add_message::<SpawnDamagePopup>()
            .add_systems(
                Update,
                (spawn_damage_popups, animate_damage_popups)
                    .chain()
                    .run_if(game_is_active)
                    .run_if(in_state(ClientState::Playing)),
            );
    }
}

/// Uniquely identifies a projectile hit for popup dedupe across prediction rollback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DamageHitId {
    pub owner: Entity,
    pub slot: u16,
    pub generation: u32,
    pub target: Entity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamagePopupKind {
    Outgoing,
    Incoming,
}

#[derive(Message, Clone, Copy, Debug)]
pub struct SpawnDamagePopup {
    pub hit: DamageHitId,
    pub amount: u32,
    pub position: Vec2,
    pub kind: DamagePopupKind,
    pub is_crit: bool,
}

#[derive(Resource, Default)]
struct RecentDamageHits {
    hits: VecDeque<DamageHitId>,
}

#[derive(Component)]
struct DamagePopup {
    remaining: Timer,
    velocity: Vec2,
}

fn spawn_damage_popups(
    mut commands: Commands,
    mut requests: MessageReader<SpawnDamagePopup>,
    mut recent: ResMut<RecentDamageHits>,
    settings: Res<GameSettings>,
) {
    let feedback = &settings.combat_feedback;
    for request in requests.read() {
        if request.amount == 0 || recent.hits.contains(&request.hit) {
            continue;
        }
        if recent.hits.len() >= RECENT_HIT_CAPACITY {
            recent.hits.pop_front();
        }
        recent.hits.push_back(request.hit);

        let color = if request.is_crit {
            feedback.crit_color
        } else {
            match request.kind {
                DamagePopupKind::Outgoing => feedback.outgoing_color,
                DamagePopupKind::Incoming => feedback.incoming_color,
            }
        };

        commands.spawn((
            DamagePopup {
                remaining: Timer::from_seconds(feedback.popup_duration_seconds, TimerMode::Once),
                velocity: Vec2::new(0.0, feedback.popup_rise_speed),
            },
            Text2d::new(request.amount.to_string()),
            TextFont {
                font_size: FontSize::Px(feedback.popup_font_size),
                ..default()
            },
            TextColor(color),
            Anchor::CENTER,
            Transform::from_translation(
                (request.position + Vec2::new(0.0, feedback.popup_offset_y)).extend(DAMAGE_POPUP_Z),
            )
            .with_scale(Vec3::splat(0.48)),
        ));
    }
}

fn animate_damage_popups(
    mut commands: Commands,
    time: Res<Time>,
    mut popups: Query<(Entity, &mut DamagePopup, &mut Transform, &mut TextColor)>,
) {
    for (entity, mut popup, mut transform, mut color) in &mut popups {
        popup.remaining.tick(time.delta());
        transform.translation += (popup.velocity * time.delta_secs()).extend(0.0);
        color.0.set_alpha(1.0 - popup.remaining.fraction());
        if popup.remaining.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
