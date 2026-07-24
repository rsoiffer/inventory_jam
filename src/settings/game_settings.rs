#[cfg(not(target_family = "wasm"))]
use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::Deserialize;

use crate::weapon::protocol::WeaponKind;

#[derive(Resource, Debug, Clone, Deserialize)]
pub(crate) struct GameSettings {
    pub enemy: EnemySettings,
    pub spawner: SpawnerSettings,
    pub fragment: FragmentSettings,
    pub player: PlayerSettings,
    pub world: WorldSettings,
    pub server_camera: ServerCameraSettings,
    pub projectile: ProjectileSettings,
    pub weapons: WeaponsSettings,
    pub menu: MenuSettings,
    pub global_aristeia: GlobalAristeiaSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnemySettings {
    pub size: f32,
    pub max_health: u32,
    pub collision_radius: f32,
    pub despawn_distance_from_players: f32,
    pub move_speed: f32,
    pub wander_radius: f32,
    pub detection_radius: f32,
    pub leash_radius: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpawnerSettings {
    pub count: usize,
    pub activation_radius: f32,
    pub max_owned: usize,
    pub spawn_radius: f32,
    pub spawn_rate_per_second: f32,
    pub placement_candidate_count: usize,
    pub spawn_min_distance_from_player: f32,
    pub spawn_candidate_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FragmentSettings {
    pub pool_min_radius: f32,
    pub pool_max_radius: f32,
    pub radius: f32,
    pub lifetime_ticks: u16,
    pub player_collection_radius: f32,
    pub pull_fraction_per_tick: f32,
    pub swirl_speed_per_tick: f32,
    pub base_drop_count: u16,
    pub bonus_drops_per_aristeia: u16,
    pub maximum_drop_count: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlayerSettings {
    pub camera_decay_rate: f32,
    pub camera_scale: f32,

    #[serde(deserialize_with = "deserialize_vec3")]
    pub username_label_offset: Vec3,

    pub aim_stick_length: f32,
    pub aim_visual_smooth_rate: f32,
    pub health_bar_width: f32,
    pub health_bar_height: f32,
    pub health_bar_offset_y: f32,
    pub spawn_attempts: usize,
    pub enemy_contact_damage: u32,
    pub enemy_contact_damage_interval_seconds: f32,
    pub half_size: f32,
    pub collision_radius: f32,
    pub move_speed: f32,
    #[allow(unused)]
    pub held_weapon_offset: f32,
    pub aristeia_duration_ticks: u16,
    pub aristeia_bar_width: f32,
    pub aristeia_bar_height: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorldSettings {
    pub tile_pixel_size: f32,
    pub arena_width_in_tiles: u32,
    pub arena_height_in_tiles: u32,
    pub safezone_width_in_tiles: u32,
    pub safezone_height_in_tiles: u32,
    pub stream_half_width: i32,
    pub stream_half_height: i32,
    pub arena_floor_tiles: Vec<usize>,
}

impl WorldSettings {
    pub fn arena_bounds(&self) -> Rect {
        Rect::from_center_size(
            Vec2::ZERO,
            Vec2::new(
                self.arena_width_in_tiles as f32 * self.tile_pixel_size,
                self.arena_height_in_tiles as f32 * self.tile_pixel_size,
            ),
        )
    }

    pub fn safezone_bounds(&self) -> Rect {
        let arena_width = self.arena_width_in_tiles as f32 * self.tile_pixel_size;
        let safezone_width = self.safezone_width_in_tiles as f32 * self.tile_pixel_size;
        Rect::from_center_size(
            Vec2::new(
                arena_width * 0.5 + safezone_width * 0.5 + self.tile_pixel_size * 8.0,
                0.0,
            ),
            Vec2::new(
                safezone_width,
                self.safezone_height_in_tiles as f32 * self.tile_pixel_size,
            ),
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerCameraSettings {
    pub zoom_speed: f32,
    pub min_scale: f32,
    pub max_scale: f32,
    pub pan_speed_in_screen_heights: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectileSettings {
    pub impact_lifetime_ticks: u16,
    pub spawn_gap: f32,
    pub buffer_capacity: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeaponsSettings {
    pub sword: WeaponKindSettings,
    pub spear: WeaponKindSettings,
    pub staff: WeaponKindSettings,
    pub bow: WeaponKindSettings,
    pub shuriken: WeaponKindSettings,
    pub boomerang: WeaponKindSettings,
}

impl WeaponsSettings {
    pub fn get(&self, kind: WeaponKind) -> &WeaponKindSettings {
        match kind {
            WeaponKind::Sword => &self.sword,
            WeaponKind::Spear => &self.spear,
            WeaponKind::Staff => &self.staff,
            WeaponKind::Bow => &self.bow,
            WeaponKind::Shuriken => &self.shuriken,
            WeaponKind::Boomerang => &self.boomerang,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeaponKindSettings {
    pub damage: u32,
    pub damage_per_level: u32,
    pub range: f32,
    pub range_per_level: f32,
    pub attacks_per_second: f32,
    pub attacks_per_second_per_level: f32,
    pub projectile_speed: f32,
    pub projectile_radius: f32,
}

impl WeaponKindSettings {
    fn bonus_levels(level: u32) -> u32 {
        level.max(1).saturating_sub(1)
    }

    pub fn damage_at(&self, level: u32) -> u32 {
        self.damage + Self::bonus_levels(level) * self.damage_per_level
    }

    pub fn range_at(&self, level: u32) -> f32 {
        self.range + Self::bonus_levels(level) as f32 * self.range_per_level
    }

    pub fn attacks_per_second_at(&self, level: u32) -> f32 {
        self.attacks_per_second
            + Self::bonus_levels(level) as f32 * self.attacks_per_second_per_level
    }

    fn is_valid(&self) -> bool {
        self.range > 0.0
            && self.range_per_level >= 0.0
            && self.attacks_per_second > 0.0
            && self.attacks_per_second_per_level >= 0.0
            && self.projectile_speed > 0.0
            && self.projectile_radius >= 0.0
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GlobalAristeiaSettings {
    pub threshold: u32,
    pub contribution_per_kill: u32,
    pub drain_interval_ticks: u16,
    pub drain_amount: u32,
    pub grand_champion_health: u32,
    pub grand_champion_move_speed: f32,
    pub grand_champion_detection_radius: f32,
    pub grand_champion_leash_radius: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MenuSettings {
    #[serde(deserialize_with = "deserialize_color")]
    pub normal_button: Color,

    #[serde(deserialize_with = "deserialize_color")]
    pub hovered_button: Color,

    #[serde(deserialize_with = "deserialize_color")]
    pub pressed_button: Color,
}

impl GameSettings {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, GameSettingsLoadError> {
        let path = path.as_ref();

        // wasm32-unknown-unknown has no filesystem, so the settings file is
        // embedded into the binary at compile time instead of read at runtime.
        #[cfg(target_family = "wasm")]
        let json = include_str!("../../assets/game_settings.json").to_owned();

        #[cfg(not(target_family = "wasm"))]
        let json = fs::read_to_string(path).map_err(|source| GameSettingsLoadError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        let settings = serde_json::from_str::<GameSettings>(&json).map_err(|source| {
            GameSettingsLoadError::Parse {
                path: path.to_path_buf(),
                source,
            }
        })?;

        settings.validate()?;
        Ok(settings)
    }
    fn validate(&self) -> Result<(), GameSettingsLoadError> {
        if self.enemy.size <= 0.0 || self.enemy.collision_radius <= 0.0 {
            return Err(GameSettingsLoadError::Invalid(
                "enemy size and collision radius must be greater than zero".into(),
            ));
        }
        if self.enemy.move_speed <= 0.0
            || self.enemy.detection_radius <= 0.0
            || self.enemy.despawn_distance_from_players <= 0.0
            || self.enemy.leash_radius < self.enemy.detection_radius
            || self.enemy.leash_radius < self.enemy.wander_radius
        {
            return Err(GameSettingsLoadError::Invalid(
                "enemy movement settings are invalid".into(),
            ));
        }
        if self.spawner.count == 0
            || self.spawner.activation_radius <= 0.0
            || self.spawner.max_owned == 0
            || self.spawner.spawn_radius <= 0.0
            || self.spawner.spawn_rate_per_second <= 0.0
            || self.spawner.placement_candidate_count == 0
            || self.spawner.spawn_candidate_count == 0
            || self.spawner.spawn_min_distance_from_player < 0.0
            || self.enemy.despawn_distance_from_players
                <= self.spawner.spawn_min_distance_from_player
        {
            return Err(GameSettingsLoadError::Invalid(
                "spawner settings are invalid".into(),
            ));
        }
        if self.fragment.pool_min_radius > self.fragment.pool_max_radius {
            return Err(GameSettingsLoadError::Invalid(
                "fragment pool_min_radius cannot exceed pool_max_radius".into(),
            ));
        }
        if !(0.0..=1.0).contains(&self.fragment.pull_fraction_per_tick) {
            return Err(GameSettingsLoadError::Invalid(
                "fragment pull_fraction_per_tick must be between 0 and 1".into(),
            ));
        }
        if self.fragment.base_drop_count > self.fragment.maximum_drop_count
            || self.global_aristeia.threshold == 0
            || self.global_aristeia.drain_interval_ticks == 0
        {
            return Err(GameSettingsLoadError::Invalid(
                "fragment drop or global Aristeia settings are invalid".into(),
            ));
        }
        if self.world.tile_pixel_size <= 0.0
            || self.world.arena_width_in_tiles == 0
            || self.world.arena_height_in_tiles == 0
            || self.world.safezone_width_in_tiles == 0
            || self.world.safezone_height_in_tiles == 0
        {
            return Err(GameSettingsLoadError::Invalid(
                "world tile size and dimensions must be greater than zero".into(),
            ));
        }
        if self.world.arena_floor_tiles.is_empty() {
            return Err(GameSettingsLoadError::Invalid(
                "world arena_floor_tiles cannot be empty".into(),
            ));
        }
        if self.player.camera_scale <= 0.0 {
            return Err(GameSettingsLoadError::Invalid(
                "player camera_scale must be greater than zero".into(),
            ));
        }
        if self.server_camera.min_scale <= 0.0
            || self.server_camera.min_scale > self.server_camera.max_scale
        {
            return Err(GameSettingsLoadError::Invalid(
                "server camera scale range is invalid".into(),
            ));
        }
        if self.projectile.buffer_capacity == 0
            || self.projectile.impact_lifetime_ticks == 0
            || self.projectile.spawn_gap < 0.0
        {
            return Err(GameSettingsLoadError::Invalid(
                "projectile buffer capacity, impact lifetime, and spawn gap are invalid".into(),
            ));
        }
        for weapon in [
            &self.weapons.sword,
            &self.weapons.spear,
            &self.weapons.staff,
            &self.weapons.bow,
            &self.weapons.shuriken,
            &self.weapons.boomerang,
        ] {
            if !weapon.is_valid() {
                return Err(GameSettingsLoadError::Invalid(
                    "weapon settings are invalid".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum GameSettingsLoadError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
    Invalid(String),
}

impl std::fmt::Display for GameSettingsLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "failed to read settings file {}: {source}",
                    path.display()
                )
            }
            Self::Parse { path, source } => {
                write!(
                    formatter,
                    "failed to parse settings file {}: {source}",
                    path.display()
                )
            }
            Self::Invalid(message) => {
                write!(formatter, "invalid game settings: {message}")
            }
        }
    }
}

impl std::error::Error for GameSettingsLoadError {}

fn deserialize_vec3<'de, D>(deserializer: D) -> Result<Vec3, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let [x, y, z] = <[f32; 3]>::deserialize(deserializer)?;
    Ok(Vec3::new(x, y, z))
}

fn deserialize_color<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let [red, green, blue] = <[f32; 3]>::deserialize(deserializer)?;
    Ok(Color::srgb(red, green, blue))
}
