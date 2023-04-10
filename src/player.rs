use std::{collections::VecDeque, time::Duration};

use crate::{
    level::{
        CurrentMetaLevel, Goal, IsActive, Lava, LevelPosition, LevelRespawnCountdown,
        LevelSpawnCountdown, MetaGridCoords, ReloadLevelEvent, TileType,
    },
    util::grid_coords_to_tile_pos,
    GameState, GRID_SIZE,
};
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_ecs_tilemap::tiles::TileStorage;
use bevy_tweening::{lens::TransformPositionLens, *};
use leafwing_input_manager::prelude::*;

const MOVEMENT_TIME_SEC: f32 = 0.1;
const NEIGHBOR_DELAY_SEC: f32 = 0.075;

const PLAYER_HAPPY_INDEX: usize = 30;
const PLAYER_NEUTRAL_INDEX: usize = 31;
const PLAYER_UNHAPPY_INDEX: usize = 32;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(InputManagerPlugin::<PlayerAction>::default())
            .init_resource::<QueuedInput>()
            .register_ldtk_entity::<PlayerBundle>("Player")
            .add_event::<TryMovePlayerEvent>()
            .add_event::<TryMoveNeighboringPlayersEvent>()
            .add_systems(
                (
                    reload_level_input,
                    add_components_to_primary_player,
                    unlock_player_movement,
                    player_face,
                )
                    .in_set(OnUpdate(GameState::InGame)),
            )
            .add_systems(
                (
                    send_try_move_event_on_input.run_if(
                        any_with_component::<PrimaryPlayer>()
                            .and_then(not(resource_exists::<LevelSpawnCountdown>()))
                            .and_then(not(resource_exists::<LevelRespawnCountdown>())),
                    ),
                    try_move_player,
                    try_move_neighboring_players,
                    process_queued_movement,
                )
                    .chain()
                    .distributive_run_if(resource_exists::<CurrentMetaLevel>())
                    .in_set(OnUpdate(GameState::InGame)),
            );
    }
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum PlayerAction {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    ResetLevel,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn unit_vec(&self) -> Vec2 {
        match self {
            Self::Up => Vec2::Y,
            Self::Down => -Vec2::Y,
            Self::Right => Vec2::X,
            Self::Left => -Vec2::X,
        }
    }

    fn unit_grid_coords(&self) -> GridCoords {
        match self {
            Self::Up => GridCoords::new(0, 1),
            Self::Down => GridCoords::new(0, -1),
            Self::Right => GridCoords::new(1, 0),
            Self::Left => GridCoords::new(-1, 0),
        }
    }
}

struct QueuedMovement {
    direction: Direction,
    delay: Timer,
}

// ===================
// ==== RESOURCES ====
// ===================

#[derive(Resource, Default)]
pub struct QueuedInput(pub VecDeque<Direction>);

// ================
// ==== EVENTS ====
// ================

pub struct TryMovePlayerEvent {
    pub player: Entity,
    pub direction: Direction,
}

pub struct TryMoveNeighboringPlayersEvent {
    pub grid_coords: MetaGridCoords,
    pub direction: Direction,
}

// ====================
// ==== COMPONENTS ====
// ====================

#[derive(Component, Default)]
pub struct Player;

#[derive(Component, Default)]
pub struct PrimaryPlayer;

#[derive(Component)]
pub struct IsMoving;

#[derive(Component, Default)]
pub struct QueuedMovements(VecDeque<QueuedMovement>);

#[derive(Bundle, LdtkEntity)]
pub struct PlayerBundle {
    player: Player,
    #[grid_coords]
    grid_coords: GridCoords,
    #[sprite_sheet_bundle]
    #[bundle]
    sprite_sheet: SpriteSheetBundle,
    #[from_entity_instance]
    entity_instance: EntityInstance,
    #[bundle]
    queued_movements: QueuedMovements,
}

// =================
// ==== SYSTEMS ====
// =================

fn add_components_to_primary_player(
    mut commands: Commands,
    player_query: Query<(Entity, &EntityInstance), Added<Player>>,
) {
    for (entity, instance) in &player_query {
        if instance.field_instances.iter().any(|field| {
            field.identifier == "Primary" && matches!(field.value, FieldValue::Bool(true))
        }) {
            commands
                .entity(entity)
                .insert(PrimaryPlayer)
                .insert(InputManagerBundle {
                    action_state: ActionState::default(),
                    input_map: InputMap::new([
                        (KeyCode::Left, PlayerAction::MoveLeft),
                        (KeyCode::Right, PlayerAction::MoveRight),
                        (KeyCode::Up, PlayerAction::MoveUp),
                        (KeyCode::Down, PlayerAction::MoveDown),
                        (KeyCode::R, PlayerAction::ResetLevel),
                    ]),
                    ..default()
                });
        }
    }
}

fn reload_level_input(
    mut reload_events: EventWriter<ReloadLevelEvent>,
    primary_players: Query<&ActionState<PlayerAction>, With<PrimaryPlayer>>,
) {
    for action_state in &primary_players {
        if action_state.just_pressed(PlayerAction::ResetLevel) {
            reload_events.send(ReloadLevelEvent);
        }
    }
}

fn send_try_move_event_on_input(
    mut queued_input: ResMut<QueuedInput>,
    primary_players: Query<
        (Entity, &ActionState<PlayerAction>, Option<&IsMoving>),
        With<PrimaryPlayer>,
    >,
    mut event_writer: EventWriter<TryMovePlayerEvent>,
) {
    let (entity, action_state, maybe_is_moving) = primary_players.single();
    let direction = queued_input.0.pop_front().or_else(|| {
        if action_state.just_pressed(PlayerAction::MoveUp) {
            Some(Direction::Up)
        } else if action_state.just_pressed(PlayerAction::MoveDown) {
            Some(Direction::Down)
        } else if action_state.just_pressed(PlayerAction::MoveRight) {
            Some(Direction::Right)
        } else if action_state.just_pressed(PlayerAction::MoveLeft) {
            Some(Direction::Left)
        } else {
            None
        }
    });
    if let Some(direction) = direction {
        if maybe_is_moving.is_some() {
            queued_input.0.push_back(direction);
        } else {
            event_writer.send(TryMovePlayerEvent {
                direction,
                player: entity,
            });
        }
    }
}

fn unlock_player_movement(
    mut commands: Commands,
    mut players: Query<
        (
            Entity,
            &Parent,
            &Animator<Transform>,
            Option<&PrimaryPlayer>,
        ),
        With<IsMoving>,
    >,
    mut levels: Query<&mut IsActive>,
) {
    for (entity, parent, animator, maybe_primary) in &mut players {
        if animator.tweenable().progress() == 1. {
            commands.entity(entity).remove::<IsMoving>();
            if maybe_primary.is_none() {
                let mut is_active = levels
                    .get_mut(parent.get())
                    .expect("player parent is a level");
                is_active.0 = false;
            }
        }
    }
}

fn process_queued_movement(
    mut commands: Commands,
    time: Res<Time>,
    mut entities: Query<(Entity, &Transform, &mut QueuedMovements)>,
) {
    for (entity, transform, mut queued_movements) in &mut entities {
        let mut to_remove = vec![];
        for (i, movement) in queued_movements.0.iter_mut().enumerate() {
            if !movement.delay.tick(time.delta()).just_finished() {
                continue;
            }
            to_remove.push(i);
            let delta = movement.direction.unit_vec().extend(0.) * GRID_SIZE as f32;
            let tween = Tween::new(
                EaseFunction::QuadraticInOut,
                Duration::from_secs_f32(MOVEMENT_TIME_SEC),
                TransformPositionLens {
                    start: transform.translation,
                    end: transform.translation + delta,
                },
            );
            commands
                .entity(entity)
                .insert((IsMoving, Animator::new(tween)));
        }
        for i in to_remove.into_iter().rev() {
            queued_movements.0.remove(i);
        }
    }
}

fn player_face(
    mut players: Query<(&Parent, &mut TextureAtlasSprite, &GridCoords), With<Player>>,
    goals: Query<(&Parent, &Goal)>,
    lavas: Query<(&Parent, &GridCoords), With<Lava>>,
    layers: Query<&Parent, With<LayerMetadata>>,
) {
    for (player_parent, mut player_sprite, player_coords) in &mut players {
        if lavas.iter().any(|(lava_parent, lava_coords)| {
            let layer_parent = layers
                .get(lava_parent.get())
                .expect("goal parent is a layer");
            layer_parent.get() == player_parent.get() && player_coords == lava_coords
        }) {
            player_sprite.index = PLAYER_UNHAPPY_INDEX;
            continue;
        }
        let (_, goal) = goals
            .iter()
            .find(|(goal_parent, _)| {
                let layer_parent = layers
                    .get(goal_parent.get())
                    .expect("goal parent is a layer");
                layer_parent.get() == player_parent.get()
            })
            .expect("every level with a player has a goal");
        if goal.activated {
            player_sprite.index = PLAYER_HAPPY_INDEX;
        } else {
            player_sprite.index = PLAYER_NEUTRAL_INDEX;
        }
    }
}

fn try_move_player(
    mut move_player_events: EventReader<TryMovePlayerEvent>,
    mut move_neighboring_players_events: EventWriter<TryMoveNeighboringPlayersEvent>,
    players: Query<(Entity, &Parent), With<PrimaryPlayer>>,
    levels: Query<(Entity, &Children, &LevelPosition)>,
    layers: Query<(&LayerMetadata, &TileStorage)>,
    tiles: Query<&TileType>,
    mut grid_coords: Query<&mut GridCoords>,
    mut queued_movements: Query<&mut QueuedMovements>,
) {
    for event in move_player_events.iter() {
        let (player_entity, parent) = players.get(event.player).unwrap();
        let (_, level_children, level_pos) = levels.get(parent.get()).unwrap();
        let (_, tile_storage) = level_children
            .iter()
            .filter_map(|&child| layers.get(child).ok())
            .find(|(metadata, _)| metadata.identifier == "TileData")
            .expect("TileData layer exists");

        let did_move = player_movement_logic(
            &tile_storage,
            &tiles,
            player_entity,
            &event.direction,
            &mut grid_coords,
        );
        if let Some(move_results) = did_move {
            for result in move_results {
                queued_movements
                    .get_mut(result.entity)
                    .expect("entity can queue movements")
                    .0
                    .push_back(QueuedMovement {
                        direction: result.direction,
                        delay: Timer::from_seconds(0., TimerMode::Once),
                    });
            }
            move_neighboring_players_events.send(TryMoveNeighboringPlayersEvent {
                grid_coords: level_pos.0,
                direction: event.direction,
            })
        }
    }
}

fn try_move_neighboring_players(
    mut move_neighboring_player_events: EventReader<TryMoveNeighboringPlayersEvent>,
    mut player_query: Query<Entity, With<Player>>,
    mut levels: Query<(&Children, &LevelPosition, &mut IsActive), With<Handle<LdtkLevel>>>,
    layers: Query<(&LayerMetadata, &TileStorage)>,
    tiles: Query<&TileType>,
    mut grid_coords: Query<&mut GridCoords>,
    mut queued_movements: Query<&mut QueuedMovements>,
) {
    for event in move_neighboring_player_events.iter() {
        for (level_children, _, mut _level_is_active) in levels
            .iter_mut()
            .filter(|(_, level_pos, _)| level_pos.0.is_neighbor(event.grid_coords))
        {
            let (_, tile_storage) = level_children
                .iter()
                .filter_map(|&child| layers.get(child).ok())
                .find(|(metadata, _)| metadata.identifier == "TileData")
                .expect("TileData layer exists");
            for &child in level_children.iter() {
                if let Ok(player) = player_query.get_mut(child) {
                    let did_move = player_movement_logic(
                        &tile_storage,
                        &tiles,
                        player,
                        &event.direction,
                        &mut grid_coords,
                    );
                    if let Some(move_results) = did_move {
                        for result in move_results {
                            queued_movements
                                .get_mut(result.entity)
                                .expect("entity can queue movements")
                                .0
                                .push_back(QueuedMovement {
                                    direction: result.direction,
                                    delay: Timer::from_seconds(NEIGHBOR_DELAY_SEC, TimerMode::Once),
                                });
                            // level_is_active.0 = true;
                        }
                    }
                }
            }
        }
    }
}

struct MoveResult {
    entity: Entity,
    direction: Direction,
}

fn player_movement_logic(
    tile_storage: &TileStorage,
    tiles: &Query<&TileType>,
    player_entity: Entity,
    direction: &Direction,
    grid_coords: &mut Query<&mut GridCoords>,
) -> Option<Vec<MoveResult>> {
    let old_coords = *grid_coords
        .get(player_entity)
        .expect("player has GridCoords");
    let new_coords = old_coords + direction.unit_grid_coords();
    let new_tile_pos = grid_coords_to_tile_pos(new_coords)?;
    let tile_entity = tile_storage.checked_get(&new_tile_pos)?;
    let tile_type = tiles.get(tile_entity).expect("tile entity is a tile");
    match tile_type {
        TileType::Wall | TileType::Boundary => return None,
        _ => {}
    }
    *grid_coords.get_mut(player_entity).unwrap() = new_coords;
    Some(vec![MoveResult {
        entity: player_entity,
        direction: *direction,
    }])
}
