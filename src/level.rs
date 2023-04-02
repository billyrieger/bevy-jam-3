use crate::{loading::GameAssets, player::Player, GameState};
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use bevy_rapier2d::prelude::*;

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LdtkSettings {
            level_spawn_behavior: LevelSpawnBehavior::UseZeroTranslation,
            ..default()
        })
        .insert_resource(RapierConfiguration {
            gravity: Vec2::ZERO,
            ..default()
        })
        .register_ldtk_entity::<DragAreaBundle>("DragArea")
        .register_ldtk_int_cell::<FloorBundle>(1)
        .register_ldtk_int_cell::<GoalBundle>(2)
        .add_system(setup.in_schedule(OnEnter(GameState::InGame)))
        .add_systems(
            (add_goal_sensor, end_game_on_goal, debug).in_set(OnUpdate(GameState::InGame)),
        );
    }
}

// ===================
// ==== RESOURCES ====
// ===================

pub struct ActiveLevel {
    drag_areas: Vec<Entity>,
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

#[derive(Component, Default)]
pub struct DragArea {
    grid_x: i32,
    grid_y: i32,
    grid_width: i32,
    grid_height: i32,
}

impl From<&EntityInstance> for DragArea {
    fn from(instance: &EntityInstance) -> Self {
        Self {
            grid_x: instance.grid.x,
            grid_y: instance.grid.y,
            grid_width: instance.width / crate::GRID_SIZE,
            grid_height: instance.height / crate::GRID_SIZE,
        }
    }
}

#[derive(Bundle, LdtkEntity)]
pub struct DragAreaBundle {
    #[from_entity_instance]
    drag_area: DragArea,
    #[from_entity_instance]
    entity_instance: EntityInstance,
}

// =================
// ==== SYSTEMS ====
// =================

fn setup(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn(LdtkWorldBundle {
        ldtk_handle: game_assets.levels.clone(),
        level_set: LevelSet {
            iids: ["ae32b950-c640-11ed-9fa7-eb373d027e17"]
                .into_iter()
                .map(String::from)
                .collect(),
        },
        transform: Transform::from_translation(Vec3::new(
            -crate::WIDTH as f32 / 2.,
            -crate::HEIGHT as f32 / 2.,
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

fn debug(
    input: Res<Input<KeyCode>>,
    drag_area_query: Query<&DragArea>,
    mut commands: Commands,
    mut layer_query: Query<(&LayerMetadata, &mut TileStorage)>,
    mut tile_query: Query<&mut TilePos>,
) {
    if input.just_pressed(KeyCode::Space) {
        let drag_areas: Vec<_> = drag_area_query.iter().collect();
        for (metadata, mut tile_storage) in &mut layer_query {
            // if metadata.identifier == "Tiles" {
                swap_tiles(&mut commands, &mut tile_query, &mut tile_storage, &drag_areas[0], &drag_areas[2]);
            // }
        }
    }
}

fn swap_tiles(mut commands: &mut Commands, tile_query: &mut Query<&mut TilePos>, tile_storage: &mut TileStorage, from_area: &DragArea, to_area: &DragArea) {
    assert_eq!(from_area.grid_width, to_area.grid_width);
    assert_eq!(from_area.grid_height, to_area.grid_height);
    for dx in 0..from_area.grid_width {
        for dy in 0..from_area.grid_height {
            let from_coords = TilePos::new(
                (from_area.grid_x + dx) as u32,
                (crate::GRID_HEIGHT - (from_area.grid_y + dy) - 1) as u32,
            );
            let to_coords = TilePos::new(
                (to_area.grid_x + dx) as u32,
                (crate::GRID_HEIGHT - (to_area.grid_y + dy) - 1) as u32,
            );
            dbg!(from_coords, to_coords);
            let from_entity = tile_storage.get(&from_coords).unwrap();
            let to_entity = tile_storage.get(&to_coords).unwrap();
            // tile_storage.remove(&to_coords);
            // tile_storage.remove(&from_coords);
            // tile_storage.set(&to_coords, from_entity);
            // tile_storage.set(&from_coords, to_entity);
            *tile_query.get_mut(from_entity).unwrap() = to_coords;
            *tile_query.get_mut(to_entity).unwrap() = from_coords;
        }
    }
}
