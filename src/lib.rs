pub mod boundary;
pub mod level;
pub mod loading;
pub mod menu;
pub mod player;
pub mod ui;
pub mod util;

use bevy::{prelude::*, render::view::RenderLayers};
use bevy_ecs_ldtk::prelude::*;
#[cfg(not(debug_assertions))]
use bevy_embedded_assets::EmbeddedAssetPlugin;
use bevy_particle_systems::ParticleSystemPlugin;
use bevy_tweening::*;

pub const WIDTH: i32 = 960;
pub const HEIGHT: i32 = 720;
pub const GRID_SIZE: i32 = 32;

pub const MAIN_RENDER_LAYER: u8 = 0;
pub const DRAG_RENDER_LAYER: u8 = 1;

const STARTING_LEVEL: i32 = 0;

const Z_OFFSET_PLAYER: f32 = 10.;
const Z_OFFSET_PARTICLE: f32 = 15.;
const Z_OFFSET_UI: f32 = 25.;

#[derive(States, Clone, Default, Debug, PartialEq, Eq, Hash)]
enum GameState {
    #[default]
    Loading,
    MainMenu,
    InGame,
    GameOver,
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
            .insert_resource(ClearColor(Color::WHITE))
            // third-party plugins
            .add_plugin(LdtkPlugin)
            .add_plugin(TweeningPlugin)
            .add_plugin(ParticleSystemPlugin)
            // .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
            // .add_plugin(bevy::diagnostic::LogDiagnosticsPlugin::default())
            // game stuff
            .add_state::<GameState>()
            .add_plugin(util::UtilPlugin)
            .add_plugin(loading::LoadingPlugin)
            .add_plugin(menu::MenuPlugin)
            .add_plugin(level::LevelPlugin)
            .add_plugin(player::PlayerPlugin)
            .add_plugin(ui::UiPlugin)
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
    commands.spawn((
        MainCamera,
        Camera2dBundle::default(),
        RenderLayers::from_layers(&[MAIN_RENDER_LAYER, DRAG_RENDER_LAYER]),
    ));
}
