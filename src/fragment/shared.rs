use bevy::prelude::*;
use rand::Rng;

use crate::fragment::protocol::Fragment;

pub const FRAGMENT_LIFETIME_TICKS: u16 = 600;
pub const FRAGMENT_RADIUS: f32 = 8.0;
pub const FRAGMENT_SPAWN_OFFSET: f32 = 25.0 + FRAGMENT_RADIUS + 1.0;

#[derive(Component, Clone, Copy, Debug, PartialEq, Deref, DerefMut)]
pub struct FragmentPosition(pub Vec2);

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct FragmentLifetime {
    pub remaining_ticks: u16,
}

#[derive(Component)]
pub struct ServerFragment;

#[derive(Component)]
pub struct ClientFragmentVisual;

pub fn fragment_spawn_position(spawn_position: Vec2) -> Vec2 {
    let mut rng = rand::rng();
    let range = -FRAGMENT_SPAWN_OFFSET..FRAGMENT_SPAWN_OFFSET;

    spawn_position + Vec2::new(rng.random_range(range.clone()), rng.random_range(range))
}
