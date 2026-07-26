use std::collections::HashMap;

use bevy::prelude::{Entity, Vec2};

use crate::protocol::rooms::{GameRoom, GameRooms};

use super::protocol::{ProjectileHitHistory, overlaps};

pub const SPATIAL_CELL_SIZE: f32 = 32.0;

#[derive(Clone, Copy, Debug)]
struct SpatialEntry {
    entity_bits: u64,
    position: Vec2,
    radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct CellKey {
    room: GameRooms,
    x: i32,
    y: i32,
}

#[derive(Clone, Debug)]
pub struct SpatialHash {
    inv_cell_size: f32,
    max_radius: f32,
    cells: HashMap<CellKey, Vec<SpatialEntry>>,
}

impl Default for SpatialHash {
    fn default() -> Self {
        Self::new(SPATIAL_CELL_SIZE)
    }
}

impl SpatialHash {
    pub fn new(cell_size: f32) -> Self {
        assert!(cell_size > 0.0);
        Self {
            inv_cell_size: 1.0 / cell_size,
            max_radius: 0.0,
            cells: HashMap::new(),
        }
    }

    pub fn insert(&mut self, room: GameRoom, position: Vec2, entity: Entity, radius: f32) {
        self.max_radius = self.max_radius.max(radius);
        let key = self.cell_key(room.room, position);
        self.cells.entry(key).or_default().push(SpatialEntry {
            entity_bits: entity.to_bits(),
            position,
            radius,
        });
    }

    pub fn find_hit(
        &self,
        room: &GameRoom,
        position: Vec2,
        projectile_radius: f32,
        hits: &ProjectileHitHistory,
    ) -> Option<u64> {
        let search = projectile_radius + self.max_radius;
        let min = (position - Vec2::splat(search)) * self.inv_cell_size;
        let max = (position + Vec2::splat(search)) * self.inv_cell_size;
        let min_x = min.x.floor() as i32;
        let max_x = max.x.floor() as i32;
        let min_y = min.y.floor() as i32;
        let max_y = max.y.floor() as i32;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let Some(entries) = self.cells.get(&CellKey {
                    room: room.room,
                    x,
                    y,
                }) else {
                    continue;
                };
                for entry in entries {
                    if hits.contains(entry.entity_bits)
                        || !overlaps(position, entry.position, projectile_radius, entry.radius)
                    {
                        continue;
                    }
                    return Some(entry.entity_bits);
                }
            }
        }
        None
    }

    fn cell_key(&self, room: GameRooms, position: Vec2) -> CellKey {
        let cell = position * self.inv_cell_size;
        CellKey {
            room,
            x: cell.x.floor() as i32,
            y: cell.y.floor() as i32,
        }
    }
}

/// Rebuild from the targets this projectile pass can collide with.
pub fn build_combat_spatial_hash(
    targets: impl IntoIterator<Item = (Entity, Vec2, GameRoom, f32)>,
) -> SpatialHash {
    let mut hash = SpatialHash::new(SPATIAL_CELL_SIZE);
    for (entity, position, room, radius) in targets {
        hash.insert(room, position, entity, radius);
    }
    hash
}
