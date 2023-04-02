use crate::{loading::GameAssets, player::Player, GameState};
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LdtkSettings {
            level_spawn_behavior: LevelSpawnBehavior::UseWorldTranslation {
                load_level_neighbors: false,
            },
            ..default()
        })
        .insert_resource(RapierConfiguration {
            gravity: Vec2::ZERO,
            ..default()
        })
        .register_ldtk_int_cell::<FloorBundle>(1)
        .register_ldtk_int_cell::<GoalBundle>(2)
        .add_system(setup.in_schedule(OnEnter(GameState::InGame)))
        .add_systems(
            (add_goal_sensor, end_game_on_goal, debug).in_set(OnUpdate(GameState::InGame)),
        );
    }
}

// ====================
// ==== COMPONENTS ====
// ====================

#[derive(Component, Default)]
pub struct Floor;

#[derive(Bundle, LdtkIntCell)]
pub struct FloorBundle {
    floor: Floor,
}

#[derive(Component, Default)]
pub struct Goal;

#[derive(Bundle, LdtkIntCell)]
pub struct GoalBundle {
    goal: Goal,
}

// =================
// ==== SYSTEMS ====
// =================

fn setup(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn(LdtkWorldBundle {
        ldtk_handle: game_assets.levels.clone(),
        level_set: LevelSet {
            iids: ["06c46f00-c640-11ed-9b09-6fc249073899"]
                .into_iter()
                .map(String::from)
                .collect(),
        },
        transform: Transform::from_translation(Vec3::new(
            -crate::WIDTH / 2.,
            -crate::HEIGHT / 2.,
            0.,
        )),
        ..default()
    });
}

fn add_goal_sensor(mut commands: Commands, goal_query: Query<Entity, Added<Goal>>) {
    for entity in &goal_query {
        commands.entity(entity).insert((
            Collider::cuboid(8., 8.),
            Sensor,
            ActiveEvents::COLLISION_EVENTS,
        ));
    }
}

fn end_game_on_goal(
    player_query: Query<&Player>,
    goal_query: Query<&Goal>,
    mut world_query: Query<&mut LevelSet>,
    mut collision_events: EventReader<CollisionEvent>,
) {
    for event in collision_events.iter() {
        match *event {
            CollisionEvent::Started(a, b, _) => {
                if player_query.contains(a) && goal_query.contains(b)
                    || player_query.contains(b) && goal_query.contains(a)
                {
                    let mut level_set = world_query.single_mut();
                    *level_set = LevelSet::default();
                }
            }
            CollisionEvent::Stopped(_, _, _) => {}
        }
    }
}

fn debug(input: Res<Input<KeyCode>>, query: Query<&GridCoords, (With<GridCoords>, With<Goal>)>) {
    if input.just_pressed(KeyCode::Space) {
        for q in &query {
            dbg!(q);
        }
    }
}
