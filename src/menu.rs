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
                "Beside Yourself",
                TextStyle {
                    font: game_assets.main_font.clone(),
                    font_size: 120.,
                    color: Color::rgb(0.1, 0.1, 0.1),
                },
            ));
            parent
                .spawn(PlayButton)
                .insert(ButtonBundle {
                    style: Style {
                        size: Size::new(Val::Px(120.), Val::Px(50.)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: Color::rgb(0.3, 0.3, 0.3).into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Play",
                        TextStyle {
                            font: game_assets.main_font.clone(),
                            font_size: 40.,
                            color: Color::rgb(0.9, 0.9, 0.9),
                        },
                    ));
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
                *color = Color::rgb(0.2, 0.2, 0.2).into();
            }
            Interaction::None => {
                *color = Color::rgb(0.3, 0.3, 0.3).into();
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
