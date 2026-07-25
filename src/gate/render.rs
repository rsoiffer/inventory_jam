use crate::app::game_is_active;
use crate::gate::{GateKind, GateOpen, GatePosition, GateProgress};
use crate::settings::GameSettings;
use bevy::prelude::*;
use bevy::sprite::Anchor;

pub struct GateRenderPlugin;

impl Plugin for GateRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_gate_visual_assets);
        app.add_systems(
            Update,
            (
                ensure_gate_sprites,
                ensure_gate_progress_bars,
                sync_gate_sprites,
                sync_gate_progress_bars,
                animate_gate_sprites,
            )
                .chain()
                .run_if(game_is_active),
        );
    }
}

#[derive(Resource)]
struct GateVisualAssets {
    open_image: Handle<Image>,
    open_layout: Handle<TextureAtlasLayout>,
    closed_image: Handle<Image>,
    open_frame_count: usize,
}

#[derive(Component)]
struct GateSpriteVisual {
    frame_count: usize,
}

#[derive(Component)]
struct GateSpriteAnimation(Timer);

#[derive(Component)]
struct GateProgressBarBackground {
    gate: Entity,
}

#[derive(Component)]
struct GateProgressBarFill {
    gate: Entity,
}

const GATE_SPRITE_Z: f32 = 3.0;
const GATE_PROGRESS_BAR_Z: f32 = 3.5;
const GATE_FRAME_PIXELS: UVec2 = UVec2::new(48, 32);
const GATE_FRAME_SECONDS: f32 = 0.14;
const GATE_OPEN_FRAME_COUNT: usize = 4;

fn load_gate_visual_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.insert_resource(GateVisualAssets {
        open_image: asset_server.load("world_tiles/spr_gate_open.png"),
        open_layout: layouts.add(TextureAtlasLayout::from_grid(
            GATE_FRAME_PIXELS,
            GATE_OPEN_FRAME_COUNT as u32,
            1,
            None,
            None,
        )),
        closed_image: asset_server.load("world_tiles/spr_gate_closed.png"),
        open_frame_count: GATE_OPEN_FRAME_COUNT,
    });
}

fn tinted_gate_sprite(assets: &GateVisualAssets, tint: Color, open: bool) -> (Sprite, usize) {
    let mut sprite = if open {
        Sprite::from_atlas_image(
            assets.open_image.clone(),
            TextureAtlas {
                layout: assets.open_layout.clone(),
                index: 0,
            },
        )
    } else {
        Sprite::from_image(assets.closed_image.clone())
    };
    sprite.color = tint;
    (sprite, if open { assets.open_frame_count } else { 1 })
}

fn ensure_gate_sprites(
    mut commands: Commands,
    assets: Res<GateVisualAssets>,
    settings: Res<GameSettings>,
    gates: Query<
        (Entity, &GateOpen),
        (
            With<GateKind>,
            With<GatePosition>,
            Without<GateSpriteVisual>,
        ),
    >,
) {
    for (entity, open) in &gates {
        let (sprite, frame_count) =
            tinted_gate_sprite(&assets, settings.post_processing.gate_tint, open.0);
        commands.entity(entity).insert((
            sprite,
            Anchor::CENTER,
            Transform {
                translation: Vec3::new(0.0, 0.0, GATE_SPRITE_Z),
                ..default()
            },
            GateSpriteVisual { frame_count },
            GateSpriteAnimation(Timer::from_seconds(
                GATE_FRAME_SECONDS,
                TimerMode::Repeating,
            )),
        ));
    }
}

fn ensure_gate_progress_bars(
    mut commands: Commands,
    settings: Res<GameSettings>,
    gates: Query<(Entity, &GatePosition, &GateKind), With<GateSpriteVisual>>,
    backgrounds: Query<&GateProgressBarBackground>,
) {
    let bar_size = Vec2::new(
        settings.gate_visual.bar_width,
        settings.gate_visual.bar_height,
    );

    for (gate, position, kind) in &gates {
        if *kind != GateKind::ToSafezone || backgrounds.iter().any(|bar| bar.gate == gate) {
            continue;
        }

        let center = position.0 + Vec2::new(0.0, settings.gate_visual.bar_offset_y);
        commands.spawn((
            GateProgressBarBackground { gate },
            Sprite::from_color(Color::srgb(0.12, 0.12, 0.14), bar_size),
            Transform::from_translation(center.extend(GATE_PROGRESS_BAR_Z)),
        ));
        commands.spawn((
            GateProgressBarFill { gate },
            Sprite::from_color(Color::srgb(0.35, 0.85, 0.55), bar_size),
            Transform::from_translation(center.extend(GATE_PROGRESS_BAR_Z + 0.1)),
        ));
    }
}

fn sync_gate_sprites(
    assets: Res<GateVisualAssets>,
    settings: Res<GameSettings>,
    mut gates: Query<(
        &GatePosition,
        &GateOpen,
        &mut Transform,
        &mut Sprite,
        &mut GateSpriteVisual,
        &mut GateSpriteAnimation,
    )>,
) {
    for (position, open, mut transform, mut sprite, mut visual, mut animation) in &mut gates {
        transform.translation = position.0.extend(GATE_SPRITE_Z);

        let showing_open = sprite.texture_atlas.is_some();
        if open.0 == showing_open {
            continue;
        }

        let (next_sprite, frame_count) =
            tinted_gate_sprite(&assets, settings.post_processing.gate_tint, open.0);
        *sprite = next_sprite;
        visual.frame_count = frame_count;
        *animation = GateSpriteAnimation(Timer::from_seconds(
            GATE_FRAME_SECONDS,
            TimerMode::Repeating,
        ));
    }
}

fn sync_gate_progress_bars(
    settings: Res<GameSettings>,
    mut commands: Commands,
    gates: Query<(&GatePosition, &GateProgress), With<GateKind>>,
    mut backgrounds: Query<
        (
            Entity,
            &GateProgressBarBackground,
            &mut Sprite,
            &mut Transform,
        ),
        Without<GateProgressBarFill>,
    >,
    mut fills: Query<
        (Entity, &GateProgressBarFill, &mut Sprite, &mut Transform),
        Without<GateProgressBarBackground>,
    >,
) {
    let full_size = Vec2::new(
        settings.gate_visual.bar_width,
        settings.gate_visual.bar_height,
    );
    let offset = Vec2::new(0.0, settings.gate_visual.bar_offset_y);

    for (entity, bar, mut sprite, mut transform) in &mut backgrounds {
        let Ok((position, _)) = gates.get(bar.gate) else {
            commands.entity(entity).despawn();
            continue;
        };
        transform.translation = (position.0 + offset).extend(GATE_PROGRESS_BAR_Z);
        sprite.custom_size = Some(full_size);
    }

    for (entity, bar, mut sprite, mut transform) in &mut fills {
        let Ok((position, progress)) = gates.get(bar.gate) else {
            commands.entity(entity).despawn();
            continue;
        };
        let width = full_size.x * progress.0.clamp(0.0, 1.0);
        let center = position.0 + offset + Vec2::new((width - full_size.x) * 0.5, 0.0);
        transform.translation = center.extend(GATE_PROGRESS_BAR_Z + 0.1);
        sprite.custom_size = Some(Vec2::new(width, full_size.y));
    }
}

fn animate_gate_sprites(
    time: Res<Time>,
    mut gates: Query<(
        &GateSpriteVisual,
        &mut GateSpriteAnimation,
        &mut Sprite,
        &GateOpen,
    )>,
) {
    for (visual, mut animation, mut sprite, open) in &mut gates {
        if !open.0 || visual.frame_count <= 1 {
            continue;
        }
        animation.0.tick(time.delta());
        if !animation.0.just_finished() {
            continue;
        }
        let Some(atlas) = sprite.texture_atlas.as_mut() else {
            continue;
        };
        atlas.index = (atlas.index + 1) % visual.frame_count;
    }
}
