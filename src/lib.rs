pub mod level;
pub mod loading;
pub mod menu;
pub mod player;
pub mod util;

use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
#[cfg(not(debug_assertions))]
use bevy_embedded_assets::EmbeddedAssetPlugin;
use bevy_rapier2d::prelude::*;

pub const WIDTH: i32 = 640;
pub const HEIGHT: i32 = 480;
pub const GRID_SIZE: i32 = 32;
pub const GRID_WIDTH: i32 = WIDTH / GRID_SIZE;
pub const GRID_HEIGHT: i32 = HEIGHT / GRID_SIZE;

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
        let default_plugins = DefaultPlugins.build().set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (WIDTH as f32, HEIGHT as f32).into(),
                canvas: Some("#bevy".to_owned()),
                ..default()
            }),
            ..default()
        });
        #[cfg(not(debug_assertions))]
        let default_plugins = default_plugins.add_before::<AssetPlugin, _>(EmbeddedAssetPlugin);
        app.add_plugins(default_plugins)
            // third-party plugins
            .add_plugin(LdtkPlugin)
            .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
            // .add_plugin(RapierDebugRenderPlugin::default())
            .insert_resource(RapierConfiguration {
                gravity: Vec2::ZERO,
                ..default()
            })
            .configure_set(LdtkSystemSet::ProcessApi.before(PhysicsSet::SyncBackend))
            // game stuff
            .add_state::<GameState>()
            .add_plugin(util::UtilPlugin)
            .add_plugin(loading::LoadingPlugin)
            .add_plugin(menu::MenuPlugin)
            .add_plugin(level::LevelPlugin)
            .add_plugin(player::PlayerPlugin)
            .add_system(setup_camera.on_startup());
    }
}

// ====================
// ==== COMPONENTS ====
// ====================

#[derive(Component)]
pub struct MainCamera;

// =================
// ==== SYSTEMS ====
// =================

fn setup_camera(mut commands: Commands) {
    commands.spawn((MainCamera, Camera2dBundle::default()));
}
