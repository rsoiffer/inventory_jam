use crate::projectile::protocol::ProjectilePosition;
use bevy::{
    app::FixedUpdate,
    color::palettes::css::RED,
    log::info,
    prelude::{App, Entity, Gizmos, Isometry2d, Plugin, Query, Res},
};
use lightyear::core::timeline::LocalTimeline;

pub struct ProjectileRenderPlugin;

impl Plugin for ProjectileRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, debug_projectile_position);
    }
}

fn debug_projectile_position(
    q: Query<(Entity, &ProjectilePosition)>,
    mut gizmos: Gizmos,
    local_timeline: Res<LocalTimeline>,
) {
    for (entity, position) in q.iter() {
        // info!(
        //     "Projectile Pos: {:?} {:?} {:?}",
        //     local_timeline.tick(),
        //     position,
        //     entity
        // );
        gizmos.circle_2d(Isometry2d::from_xy(position.x, position.y), 5.0, RED);
    }
}
