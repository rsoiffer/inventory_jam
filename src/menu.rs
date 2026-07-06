use crate::GameState;
use crate::config::{NetworkSession, default_join_server_addr};
use crate::loading::TextureAssets;
use bevy::prelude::*;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ButtonColors>()
            .register_type::<Menu>()
            .register_type::<MenuAction>()
            .register_type::<OpenLink>()
            .add_systems(OnEnter(GameState::Menu), setup_menu)
            .add_systems(Update, menu_button_colors.run_if(in_state(GameState::Menu)))
            .add_systems(Update, menu_action_buttons.run_if(in_state(GameState::Menu)))
            .add_systems(Update, menu_link_buttons.run_if(in_state(GameState::Menu)))
            .add_systems(OnExit(GameState::Menu), cleanup_menu);
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct ButtonColors {
    normal: Color,
    hovered: Color,
}

impl Default for ButtonColors {
    fn default() -> Self {
        ButtonColors {
            normal: Color::linear_rgb(0.15, 0.15, 0.15),
            hovered: Color::linear_rgb(0.25, 0.25, 0.25),
        }
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Menu;

#[derive(Component, Reflect)]
#[reflect(Component)]
enum MenuAction {
    Host,
    Join,
}

fn setup_menu(mut commands: Commands, textures: Res<TextureAssets>) {
    commands.spawn(Camera2d);
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(12.0),
                ..default()
            },
            Menu,
        ))
        .with_children(|children| {
            #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
            spawn_action_button(children, "Host", MenuAction::Host);
            spawn_action_button(children, "Join", MenuAction::Join);
        });
    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceAround,
                bottom: Val::Px(5.),
                width: Val::Percent(100.),
                position_type: PositionType::Absolute,
                ..default()
            },
            Menu,
        ))
        .with_children(|children| {
            children
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(170.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::SpaceAround,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(5.)),
                        ..Default::default()
                    },
                    BackgroundColor(Color::NONE),
                    ButtonColors {
                        normal: Color::NONE,
                        ..default()
                    },
                    OpenLink("https://bevyengine.org"),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Made with Bevy"),
                        TextFont {
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(Color::linear_rgb(0.9, 0.9, 0.9)),
                    ));
                    parent.spawn((
                        ImageNode {
                            image: textures.bevy.clone(),
                            ..default()
                        },
                        Node {
                            width: Val::Px(32.),
                            ..default()
                        },
                    ));
                });
            children
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(170.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::SpaceAround,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(5.)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    ButtonColors {
                        normal: Color::NONE,
                        hovered: Color::linear_rgb(0.25, 0.25, 0.25),
                    },
                    OpenLink("https://github.com/NiklasEi/bevy_game_template"),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Open source"),
                        TextFont {
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(Color::linear_rgb(0.9, 0.9, 0.9)),
                    ));
                    parent.spawn((
                        ImageNode::new(textures.github.clone()),
                        Node {
                            width: Val::Px(32.),
                            ..default()
                        },
                    ));
                });
        });
}

fn spawn_action_button(parent: &mut ChildSpawnerCommands, label: &str, action: MenuAction) {
    let button_colors = ButtonColors::default();
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(140.0),
                height: Val::Px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            BackgroundColor(button_colors.normal),
            button_colors,
            action,
        ))
        .with_child((
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(32.0),
                ..default()
            },
            TextColor(Color::linear_rgb(0.9, 0.9, 0.9)),
        ));
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct OpenLink(#[reflect(ignore)] &'static str);

fn menu_button_colors(
    mut buttons: Query<(&Interaction, &mut BackgroundColor, &ButtonColors), With<Button>>,
) {
    for (interaction, mut color, button_colors) in buttons.iter_mut() {
        *color = match *interaction {
            Interaction::Hovered | Interaction::Pressed => button_colors.hovered,
            Interaction::None => button_colors.normal,
        }
        .into();
    }
}

fn menu_action_buttons(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    buttons: Query<(&Interaction, &MenuAction), With<Button>>,
) {
    for (interaction, action) in buttons.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let client_id = rand::random();
        let session = match action {
            MenuAction::Host => NetworkSession::host(client_id),
            MenuAction::Join => NetworkSession::join(default_join_server_addr(), client_id),
        };
        commands.insert_resource(session);
        next_state.set(GameState::Playing);
    }
}

fn menu_link_buttons(buttons: Query<(&Interaction, &OpenLink), With<Button>>) {
    for (interaction, link) in buttons.iter() {
        if *interaction == Interaction::Pressed
            && let Err(error) = webbrowser::open(link.0)
        {
            warn!("Failed to open link {error:?}");
        }
    }
}

fn cleanup_menu(mut commands: Commands, menu: Query<Entity, With<Menu>>) {
    for entity in menu.iter() {
        commands.entity(entity).despawn();
    }
}
