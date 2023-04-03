use crate::{loading::GameAssets, player::Player, GameState};
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<LoadLevelEvent>()
            .register_ldtk_int_cell::<FloorBundle>(1)
            .register_ldtk_int_cell::<GoalBundle>(2)
            .add_system(setup.in_schedule(OnEnter(GameState::InGame)))
            .add_systems(
                (
                    add_goal_sensor,
                    end_game_on_goal,
                    load_level,
                    offset_ldtk_levels_on_spawn.run_if(resource_exists::<ActiveLevel>()),
                    debug,
                )
                    .in_set(OnUpdate(GameState::InGame)),
            );
    }
}

// ===================
// ==== RESOURCES ====
// ===================

#[derive(Resource)]
pub struct ActiveLevel(LevelData);

#[derive(Clone)]
pub struct LevelData {
    width: i32,
    height: i32,
    ldtk_level_iids: Vec<&'static str>,
}

impl LevelData {
    fn level0() -> Self {
        Self {
            width: 2,
            height: 2,
            ldtk_level_iids: vec![
                "fb209a20-c640-11ed-9fa7-2b9366b48038",
                "10b5cf40-c640-11ed-9fa7-915e1d71b678",
                "130c4260-c640-11ed-9fa7-379a4030ffd2",
                "16df4360-c640-11ed-9fa7-658babce04c5",
            ],
        }
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
// ==== EVENTS =====
// =================

struct LoadLevelEvent {
    level_data: LevelData,
}

// =================
// ==== SYSTEMS ====
// =================

fn setup(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut event_writer: EventWriter<LoadLevelEvent>,
) {
    commands.spawn(LdtkWorldBundle {
        ldtk_handle: game_assets.levels.clone(),
        level_set: LevelSet::default(),
        ..default()
    });
    event_writer.send(LoadLevelEvent {
        level_data: LevelData::level0(),
    });
}

fn load_level(
    mut commands: Commands,
    mut ldtk_world_query: Query<&mut LevelSet>,
    mut event_reader: EventReader<LoadLevelEvent>,
) {
    let mut level_set = ldtk_world_query.single_mut();
    if let Some(event) = event_reader.iter().next() {
        level_set.iids = event
            .level_data
            .ldtk_level_iids
            .iter()
            .map(|s| s.to_string())
            .collect();
        commands.insert_resource(ActiveLevel(event.level_data.clone()));
    }
    event_reader.clear();
}

fn offset_ldtk_levels_on_spawn(
    active_level: Res<ActiveLevel>,
    ldtk_level_assets: Res<Assets<LdtkLevel>>,
    mut ldtk_level_query: Query<(&Handle<LdtkLevel>, &mut Transform), Added<Handle<LdtkLevel>>>,
) {
    for (level_handle, mut transform) in &mut ldtk_level_query {
        let ldtk_level = ldtk_level_assets
            .get(&level_handle)
            .expect("ldtk level is loaded");
        let (px_wid, px_hei) = (ldtk_level.level.px_wid, ldtk_level.level.px_hei);
        // Levels are loaded with the bottom left corner at the world origin.
        // Here we offset the level so that the center of the level aligns with
        // the world origin.
        transform.translation += Vec3::new(-px_wid as f32 / 2., -px_hei as f32 / 2., 0.);
        let ldtk_level_index = active_level
            .0
            .ldtk_level_iids
            .iter()
            .position(|iid| *iid == ldtk_level.level.iid)
            .expect("level iid exists in active level");
        let row = (ldtk_level_index as i32 / active_level.0.width) as f32;
        let col = (ldtk_level_index as i32 % active_level.0.width) as f32;
        let x_offset = (col - 0.5 * (active_level.0.width as f32 - 1.)) * px_wid as f32;
        // y offset is flipped
        let y_offset = -(row - 0.5 * (active_level.0.height as f32 - 1.)) * px_hei as f32;
        transform.translation += Vec3::new(x_offset, y_offset, 0.);
    }
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

fn debug(input: Res<Input<KeyCode>>, mut event_writer: EventWriter<LoadLevelEvent>) {
    if input.just_pressed(KeyCode::Space) {
        event_writer.send(LoadLevelEvent {
            level_data: LevelData::level0(),
        })
    }
}
