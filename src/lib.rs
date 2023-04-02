pub mod level;
pub mod loading;
pub mod menu;
pub mod player;

use bevy::{ecs::{archetype::Archetypes, component::ComponentId}, prelude::*};
use bevy_ecs_ldtk::prelude::*;
use bevy_embedded_assets::EmbeddedAssetPlugin;
use bevy_rapier2d::prelude::*;

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
            // third-party plugins
            .add_plugin(LdtkPlugin)
            .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
            .add_plugin(RapierDebugRenderPlugin::default())
            .insert_resource(LdtkSettings {
                level_spawn_behavior: LevelSpawnBehavior::UseWorldTranslation {
                    load_level_neighbors: false,
                },
                ..default()
            })
            .insert_resource(RapierConfiguration {
                gravity: Vec2::ZERO,
                ..default()
            })
            .configure_set(LdtkSystemSet::ProcessApi.before(PhysicsSet::SyncBackend))
            // game plugins
            .add_plugin(loading::LoadingPlugin)
            .add_plugin(menu::MenuPlugin)
            .add_plugin(level::LevelPlugin)
            .add_plugin(player::PlayerPlugin)
            .add_system(setup.on_startup());
    }
}

pub fn get_components_for_entity<'a>(
    entity: &Entity,
    archetypes: &'a Archetypes,
) -> Option<impl Iterator<Item = ComponentId> + 'a> {
    for archetype in archetypes.iter() {
        if archetype.entities().iter().any(|e| e.entity() == *entity) {
            return Some(archetype.components());
        }
    }
    None
}

// ====================
// ==== COMPONENTS ====
// ====================

#[derive(Component)]
pub struct MainCamera;

fn setup(mut commands: Commands) {
    commands.spawn((MainCamera, Camera2dBundle::default()));
}
