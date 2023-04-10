use crate::loading::GameAssets;
use crate::GameState;
use bevy::prelude::*;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(setup_main_menu.in_schedule(OnEnter(GameState::MainMenu)))
            .add_system(cleanup_menu.in_schedule(OnExit(GameState::MainMenu)))
            .add_systems(
                (hover_buttons, play_button_on_click).in_set(OnUpdate(GameState::MainMenu)),
            );
    }
}

// ====================
// ==== COMPONENTS ====
// ====================

#[derive(Component)]
struct MainMenu;

#[derive(Component)]
struct TitleText;

#[derive(Component)]
struct PlayButton;

// =================
// ==== SYSTEMS ====
// =================

fn setup_main_menu(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands
        .spawn(MainMenu)
        .insert(NodeBundle {
            style: Style {
                size: Size::width(Val::Percent(100.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TitleText).insert(TextBundle::from_section(
                "BESIDE YOURSELF",
                TextStyle {
                    font: game_assets.main_font.clone(),
                    font_size: 120.,
                    color: Color::rgb(0.1, 0.1, 0.1),
                },
            ));
            parent
                .spawn(NodeBundle {
                    style: Style {
                        size: Size::width(Val::Percent(100.0)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        margin: UiRect::all(Val::Px(32.)),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    let lines = [
                        "Use the arrow keys or WASD to control the primary player.",
                        "Use the mouse to click and drag levels to swap their positions.",
                        "The primary player affects players in the surrounding levels.",
                        "Get all players to the goal to move to the next stage.",
                        "Press R to reset a  if you get stuck.",
                        "Good luck!",
                    ];
                    for line in lines {
                        parent.spawn(TitleText).insert(TextBundle::from_section(
                            line,
                            TextStyle {
                                font: game_assets.main_font.clone(),
                                font_size: 24.,
                                color: Color::rgb(0.1, 0.1, 0.1),
                            },
                        ));
                    }
                });
            parent
                .spawn(PlayButton)
                .insert(ButtonBundle {
                    style: Style {
                        size: Size::width(Val::Auto),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: Color::rgb(1., 1., 1.).into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(
                        TextBundle::from_section(
                            "Click here to play",
                            TextStyle {
                                font: game_assets.main_font.clone(),
                                font_size: 48.,
                                color: Color::rgb(0.1, 0.1, 0.1),
                            },
                        )
                        .with_style(Style {
                            margin: UiRect::all(Val::Px(8.)),
                            ..default()
                        }),
                    );
                });
        });
}

fn hover_buttons(
    mut button_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color) in &mut button_query {
        match *interaction {
            Interaction::Hovered => {
                *color = Color::rgb(0.7, 0.7, 0.7).into();
            }
            Interaction::None => {
                *color = Color::rgb(1., 1., 1.).into();
            }
            _ => {}
        }
    }
}

fn play_button_on_click(
    mut state: ResMut<NextState<GameState>>,
    mut button_query: Query<&Interaction, (Changed<Interaction>, With<PlayButton>)>,
) {
    for interaction in &mut button_query {
        match *interaction {
            Interaction::Clicked => {
                state.set(GameState::InGame);
            }
            _ => {}
        }
    }
}

fn cleanup_menu(mut commands: Commands, menu_query: Query<Entity, With<MainMenu>>) {
    commands.entity(menu_query.single()).despawn_recursive();
}
