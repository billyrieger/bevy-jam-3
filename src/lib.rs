pub mod level;
pub mod loading;
pub mod menu;
pub mod player;

use bevy::prelude::*;
use bevy_embedded_assets::EmbeddedAssetPlugin;

pub const WIDTH: f32 = 640.;
pub const HEIGHT: f32 = 480.;

#[derive(States, Clone, Default, Debug, PartialEq, Eq, Hash)]
enum GameState {
    #[default]
    Loading,
    MainMenu,
    InGame,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameState>()
            .add_plugins(
                DefaultPlugins
                    .build()
                    .set(WindowPlugin {
                        primary_window: Some(Window {
                            resolution: (WIDTH, HEIGHT).into(),
                            canvas: Some("#bevy".to_owned()),
                            ..default()
                        }),
                        ..default()
                    })
                    .add_before::<AssetPlugin, _>(EmbeddedAssetPlugin),
            )
            .add_plugin(loading::LoadingPlugin)
            .add_plugin(menu::MenuPlugin)
            .add_plugin(level::LevelPlugin)
            .add_plugin(player::PlayerPlugin)
            .add_system(setup.on_startup());
    }
}

// ==== COMPONENTS ====

#[derive(Component)]
pub struct MainCamera;

fn setup(mut commands: Commands) {
    commands.spawn((MainCamera, Camera2dBundle::default()));
}
