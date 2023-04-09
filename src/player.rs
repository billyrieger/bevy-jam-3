use std::collections::VecDeque;

use crate::{
    level::{
        CurrentMetaLevel, IsActive, LevelPosition, LevelSpawnCountdown, MetaGridCoords, TileType,
    },
    util::grid_coords_to_tile_pos,
    GameState, GRID_SIZE,
};
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_ecs_tilemap::tiles::TileStorage;
use bevy_rapier2d::prelude::*;
use bevy_tweening::{lens::TransformPositionLens, *};
use leafwing_input_manager::prelude::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(InputManagerPlugin::<PlayerAction>::default())
            .register_ldtk_entity::<PlayerBundle>("Player")
            .add_event::<TryMovePlayerEvent>()
            .add_event::<TryMoveNeighboringPlayersEvent>()
            .add_systems(
                (add_components_to_primary_player, unlock_player_movement)
                    .in_set(OnUpdate(GameState::InGame)),
            )
            .add_systems(
                (
                    send_try_move_event_on_input.run_if(
                        any_with_component::<PrimaryPlayer>()
                            .and_then(not(resource_exists::<LevelSpawnCountdown>())),
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
struct QueuedInput(VecDeque<Direction>);

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
    #[with(player_physics_bundle)]
    #[bundle]
    physics: (RigidBody, Collider, ActiveEvents),
    #[from_entity_instance]
    entity_instance: EntityInstance,
    #[bundle]
    queued_movements: QueuedMovements,
}

fn player_physics_bundle(_: &EntityInstance) -> (RigidBody, Collider, ActiveEvents) {
    (
        RigidBody::Dynamic,
        Collider::ball(8.),
        ActiveEvents::COLLISION_EVENTS,
    )
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

fn send_try_move_event_on_input(
    mut queued_input: Local<QueuedInput>,
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
    mut players: Query<(
        Entity,
        &Parent,
        &mut GridCoords,
        &Animator<Transform>,
        &IsMoving,
        Option<&PrimaryPlayer>,
    )>,
    mut levels: Query<&mut IsActive>,
) {
    for (entity, parent, mut grid_coords, animator, movement, maybe_primary) in &mut players {
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
    current_level: Res<CurrentMetaLevel>,
    mut entities: Query<(Entity, &Transform, &mut GridCoords, &mut QueuedMovements)>,
) {
    for (entity, transform, mut grid_coords, mut queued_movements) in &mut entities {
        if let Some(mut movement) = queued_movements.0.pop_front() {
            if !movement.delay.tick(time.delta()).just_finished() {
                queued_movements.0.push_front(movement);
                continue;
            }
            *grid_coords += movement.direction.unit_grid_coords();
            let delta = movement.direction.unit_vec().extend(0.) * GRID_SIZE as f32;
            let tween = Tween::new(
                EaseFunction::QuadraticInOut,
                std::time::Duration::from_secs_f32(0.1),
                TransformPositionLens {
                    start: transform.translation,
                    end: transform.translation + delta,
                },
            );
            commands
                .entity(entity)
                .insert((Animator::new(tween), IsMoving));
        }
    }
}

fn try_move_player(
    mut commands: Commands,
    mut move_player_events: EventReader<TryMovePlayerEvent>,
    mut move_neighboring_players_events: EventWriter<TryMoveNeighboringPlayersEvent>,
    mut player_query: Query<
        (Entity, &mut GridCoords, &mut Transform, &Parent),
        With<PrimaryPlayer>,
    >,
    level_query: Query<(Entity, &Children, &LevelPosition), With<Handle<LdtkLevel>>>,
    layer_query: Query<(&LayerMetadata, &TileStorage)>,
    tile_query: Query<&TileType>,
    mut queued_movements: Query<&mut QueuedMovements>,
) {
    for event in move_player_events.iter() {
        let (player_entity, mut player_coords, mut player_transform, parent) =
            player_query.get_mut(event.player).unwrap();
        let (_, level_children, level_pos) = level_query.get(parent.get()).unwrap();
        let (_, tile_storage) = level_children
            .iter()
            .filter_map(|&child| layer_query.get(child).ok())
            .find(|(metadata, _)| metadata.identifier == "TileData")
            .expect("TileData layer exists");

        let did_move = player_movement_logic(
            &mut commands,
            &tile_storage,
            &tile_query,
            player_entity,
            &mut player_transform.translation,
            &mut player_coords,
            &event.direction,
        );
        if did_move.is_some() {
            queued_movements
                .get_mut(player_entity)
                .expect("player entity can queue movements")
                .0
                .push_back(QueuedMovement {
                    direction: event.direction,
                    delay: Timer::from_seconds(0., TimerMode::Once),
                });
            move_neighboring_players_events.send(TryMoveNeighboringPlayersEvent {
                grid_coords: level_pos.0,
                direction: event.direction,
            })
        }
    }
}

fn try_move_neighboring_players(
    mut commands: Commands,
    mut move_neighboring_player_events: EventReader<TryMoveNeighboringPlayersEvent>,
    mut player_query: Query<(Entity, &mut GridCoords, &mut Transform), With<Player>>,
    mut levels: Query<(&Children, &LevelPosition, &mut IsActive), With<Handle<LdtkLevel>>>,
    layers: Query<(&LayerMetadata, &TileStorage)>,
    tile_query: Query<&TileType>,
    mut queued_movements: Query<&mut QueuedMovements>,
) {
    for event in move_neighboring_player_events.iter() {
        for (level_children, _, mut level_is_active) in levels
            .iter_mut()
            .filter(|(_, level_pos, _)| level_pos.0.is_neighbor(event.grid_coords))
        {
            let (_, tile_storage) = level_children
                .iter()
                .filter_map(|&child| layers.get(child).ok())
                .find(|(metadata, _)| metadata.identifier == "TileData")
                .expect("TileData layer exists");

            let player_children: Vec<Entity> = level_children
                .iter()
                .copied()
                .filter(|&child| player_query.contains(child))
                .collect();
            for player in player_children {
                if let Ok((entity, mut player_coords, mut player_transform)) =
                    player_query.get_mut(player)
                {
                    let did_move = player_movement_logic(
                        &mut commands,
                        &tile_storage,
                        &tile_query,
                        entity,
                        &mut player_transform.translation,
                        &mut player_coords,
                        &event.direction,
                    );
                    if did_move.is_some() {
                        queued_movements
                            .get_mut(player)
                            .expect("player entity can queue movements")
                            .0
                            .push_back(QueuedMovement {
                                direction: event.direction,
                                delay: Timer::from_seconds(0.1, TimerMode::Once),
                            });
                        level_is_active.0 = true;
                    }
                }
            }
        }
    }
}

fn player_movement_logic(
    commands: &mut Commands,
    tile_storage: &TileStorage,
    tiles: &Query<&TileType>,
    player_entity: Entity,
    player_translation: &mut Vec3,
    player_coords: &mut GridCoords,
    direction: &Direction,
) -> Option<()> {
    let new_coords = *player_coords + direction.unit_grid_coords();
    let new_tile_pos = grid_coords_to_tile_pos(new_coords)?;
    let tile_entity = tile_storage.checked_get(&new_tile_pos)?;
    let tile_type = tiles.get(tile_entity).expect("tile entity is a tile");
    match tile_type {
        TileType::Wall | TileType::Boundary => return None,
        _ => {}
    }
    // commands
    //     .entity(player_entity)
    //     .insert(IsMoving {
    //         from: *player_coords,
    //         to: new_coords,
    //     })
    //     .insert(Animator::new(Tween::new(
    //         EaseFunction::QuadraticInOut,
    //         std::time::Duration::from_secs_f32(0.1),
    //         TransformPositionLens {
    //             start: *player_translation,
    //             end: *player_translation
    //                 + direction.unit_vec().extend(0.) * crate::GRID_SIZE as f32,
    //         },
    //     )));
    Some(())
}
