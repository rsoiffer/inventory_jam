//! Main menu UI: username entry and connection actions.

#[cfg(feature = "server")]
use crate::app::ServerState;
use crate::app::{ClientState, LaunchMode, LocalUsername, MAX_USERNAME_LEN, StartGame};
use crate::networking::ConnectionStatus;
use crate::settings::GameSettings;
use bevy::color::palettes::tailwind::{SLATE_300, SLATE_700, SLATE_900};
use bevy::input_focus::AutoFocus;
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin};
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
struct UsernameInput;

#[derive(Component)]
struct StatusText;

#[derive(Component, Clone, Copy, Debug)]
enum MenuButton {
    Join,
    #[cfg(all(feature = "server", not(target_family = "wasm")))]
    HostLocal,
    #[cfg(all(feature = "client", not(target_family = "wasm")))]
    JoinLocal,
}

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TabNavigationPlugin);
        app.add_systems(OnEnter(ClientState::Disconnected), spawn_main_menu);
        app.add_systems(OnEnter(ClientState::Connecting), spawn_connecting_overlay);
        app.add_systems(OnExit(ClientState::Disconnected), despawn_menu_ui);
        app.add_systems(OnExit(ClientState::Connecting), despawn_menu_ui);
        #[cfg(feature = "server")]
        app.add_systems(OnEnter(ServerState::Hosting), despawn_menu_ui);
        app.add_systems(
            Update,
            (style_menu_buttons, handle_menu_buttons, sync_status_text)
                .run_if(
                    in_state(ClientState::Disconnected).or_else(in_state(ClientState::Connecting)),
                )
                .run_if(not_hosting),
        );
    }
}

fn spawn_main_menu(
    mut commands: Commands,
    settings: Res<GameSettings>,
    username: Res<LocalUsername>,
    status: Res<ConnectionStatus>,
) {
    let input = commands
        .spawn((
            UsernameInput,
            AutoFocus,
            TabIndex(0),
            Node {
                width: px(280),
                height: px(40),
                border: px(2).all(),
                padding: px(8).all(),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(Color::from(SLATE_300)),
            BackgroundColor(Color::from(SLATE_900)),
            EditableText {
                visible_width: Some(18.),
                allow_newlines: false,
                max_characters: Some(MAX_USERNAME_LEN),
                ..EditableText::new(&username.0)
            },
            TextLayout::no_wrap(),
            TextFont {
                font_size: FontSize::Px(22.0),
                ..default()
            },
            TextCursorStyle::default(),
        ))
        .id();

    let mut children = vec![
        commands
            .spawn((
                Text::new("Arena of Champions"),
                TextFont {
                    font_size: FontSize::Px(42.0),
                    ..default()
                },
            ))
            .id(),
        commands
            .spawn((
                Text::new("Username"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
            ))
            .id(),
        input,
        menu_button(&mut commands, &settings, MenuButton::Join, "Join", 1),
    ];

    #[cfg(all(feature = "server", not(target_family = "wasm")))]
    children.push(menu_button(
        &mut commands,
        &settings,
        MenuButton::HostLocal,
        "Host Local Server",
        2,
    ));
    #[cfg(all(feature = "client", not(target_family = "wasm")))]
    children.push(menu_button(
        &mut commands,
        &settings,
        MenuButton::JoinLocal,
        "Join Local Server",
        3,
    ));

    children.push(
        commands
            .spawn((
                StatusText,
                Text::new(status.message.clone()),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.75, 0.45)),
            ))
            .id(),
    );

    commands
        .spawn((
            MenuRoot,
            TabGroup::default(),
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.92)),
        ))
        .add_children(&children);
}

fn spawn_connecting_overlay(mut commands: Commands, status: Res<ConnectionStatus>) {
    spawn_status_screen(
        &mut commands,
        "Connecting...",
        if status.message.is_empty() {
            "please wait"
        } else {
            &status.message
        },
    );
}

fn spawn_status_screen(commands: &mut Commands, title: &str, detail: &str) {
    let title_id = commands
        .spawn((
            Text::new(title),
            TextFont {
                font_size: FontSize::Px(32.0),
                ..default()
            },
        ))
        .id();
    let detail_id = commands
        .spawn((
            StatusText,
            Text::new(detail.to_string()),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::srgb(0.85, 0.75, 0.45)),
        ))
        .id();
    commands
        .spawn((
            MenuRoot,
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.92)),
        ))
        .add_children(&[title_id, detail_id]);
}

fn despawn_menu_ui(mut commands: Commands, roots: Query<Entity, With<MenuRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}

#[cfg(feature = "server")]
fn not_hosting(server_state: Res<State<ServerState>>) -> bool {
    *server_state.get() != ServerState::Hosting
}

#[cfg(not(feature = "server"))]
fn not_hosting() -> bool {
    true
}

fn menu_button(
    commands: &mut Commands,
    settings: &GameSettings,
    action: MenuButton,
    label: &str,
    tab: i32,
) -> Entity {
    commands
        .spawn((
            Button,
            action,
            TabIndex(tab),
            Node {
                width: px(280),
                height: px(48),
                border: px(2).all(),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(Color::from(SLATE_700)),
            BackgroundColor(settings.menu.normal_button),
            children![(
                Text::new(label.to_string()),
                TextFont {
                    font_size: FontSize::Px(22.0),
                    ..default()
                },
            )],
        ))
        .id()
}

fn style_menu_buttons(
    settings: Res<GameSettings>,
    mut buttons: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<MenuButton>),
    >,
) {
    for (interaction, mut background, mut border) in &mut buttons {
        match *interaction {
            Interaction::Pressed => {
                *background = settings.menu.pressed_button.into();
                *border = BorderColor::all(Color::srgb(0.4, 0.9, 0.55));
            }
            Interaction::Hovered => {
                *background = settings.menu.hovered_button.into();
                *border = BorderColor::all(Color::WHITE);
            }
            Interaction::None => {
                *background = settings.menu.normal_button.into();
                *border = BorderColor::all(Color::from(SLATE_700));
            }
        }
    }
}

fn sync_status_text(status: Res<ConnectionStatus>, mut texts: Query<&mut Text, With<StatusText>>) {
    if !status.is_changed() {
        return;
    }
    for mut text in &mut texts {
        text.0.clone_from(&status.message);
    }
}

fn handle_menu_buttons(
    interactions: Query<(&Interaction, &MenuButton), Changed<Interaction>>,
    inputs: Query<&EditableText, With<UsernameInput>>,
    mut username: ResMut<LocalUsername>,
    mut status: ResMut<ConnectionStatus>,
    mut starts: MessageWriter<StartGame>,
) {
    for (interaction, button) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if let Ok(input) = inputs.single() {
            username.0 = input.value().to_string();
        }

        match button {
            MenuButton::Join => match username.validated() {
                Ok(name) => {
                    username.0.clone_from(&name);
                    starts.write(StartGame {
                        mode: LaunchMode::JoinEdgegap,
                        username: name,
                    });
                }
                Err(error) => status.message = error,
            },
            #[cfg(all(feature = "server", not(target_family = "wasm")))]
            MenuButton::HostLocal => {
                starts.write(StartGame {
                    mode: LaunchMode::DedicatedServer,
                    username: String::new(),
                });
            }
            #[cfg(all(feature = "client", not(target_family = "wasm")))]
            MenuButton::JoinLocal => match username.validated() {
                Ok(name) => {
                    username.0.clone_from(&name);
                    starts.write(StartGame {
                        mode: LaunchMode::JoinLocal,
                        username: name,
                    });
                }
                Err(error) => status.message = error,
            },
        }
    }
}
