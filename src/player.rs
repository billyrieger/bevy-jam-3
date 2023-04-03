use crate::{
    level::{LevelPosition, MetaGridPos},
    GameState,
};
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_ecs_tilemap::tiles::{TilePos, TileStorage};
use bevy_rapier2d::prelude::*;
use leafwing_input_manager::prelude::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(InputManagerPlugin::<PlayerAction>::default())
            .register_ldtk_entity::<PlayerBundle>("Player")
            .add_event::<MovePlayerEvent>()
            .add_event::<MoveNeighboringPlayersEvent>()
            .add_systems((add_components_to_primary_player,).in_set(OnUpdate(GameState::InGame)))
            .add_systems(
                (
                    send_player_move_event_on_input,
                    move_player,
                    move_neighboring_players,
                )
                    .chain()
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

// ================
// ==== EVENTS ====
// ================

pub struct MovePlayerEvent {
    pub player: Entity,
    pub direction: Direction,
}

pub struct MoveNeighboringPlayersEvent {
    pub grid_coords: MetaGridPos,
    pub direction: Direction,
}

// ====================
// ==== COMPONENTS ====
// ====================

#[derive(Component, Default)]
pub struct Player;

#[derive(Component, Default)]
pub struct PrimaryPlayer;

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
                    ]),
                    ..default()
                });
        }
    }
}

fn send_player_move_event_on_input(
    player_query: Query<(Entity, &ActionState<PlayerAction>), With<Player>>,
    mut event_writer: EventWriter<MovePlayerEvent>,
) {
    for (entity, action_state) in &player_query {
        let direction = if action_state.just_pressed(PlayerAction::MoveUp) {
            Some(Direction::Up)
        } else if action_state.just_pressed(PlayerAction::MoveDown) {
            Some(Direction::Down)
        } else if action_state.just_pressed(PlayerAction::MoveRight) {
            Some(Direction::Right)
        } else if action_state.just_pressed(PlayerAction::MoveLeft) {
            Some(Direction::Left)
        } else {
            None
        };
        if let Some(direction) = direction {
            event_writer.send(MovePlayerEvent {
                direction,
                player: entity,
            });
        }
    }
}

fn move_player(
    mut move_player_events: EventReader<MovePlayerEvent>,
    mut move_neighboring_player_events: EventWriter<MoveNeighboringPlayersEvent>,
    mut player_query: Query<
        (
            &mut GridCoords,
            &mut Transform,
            &Parent,
            Option<&PrimaryPlayer>,
        ),
        With<Player>,
    >,
    level_query: Query<(Entity, &Children, &LevelPosition), With<Handle<LdtkLevel>>>,
    layer_query: Query<(&LayerMetadata, &TileStorage)>,
) {
    for event in move_player_events.iter() {
        let (mut player_coords, mut player_transform, parent, maybe_primary_player) =
            player_query.get_mut(event.player).unwrap();
        let (_, level_children, level_pos) = level_query.get(parent.get()).unwrap();
        for &child in level_children {
            let Ok((metadata, tile_storage)) = layer_query.get(child) else { continue };
            if metadata.identifier != "Tiles" {
                continue;
            }
            let did_move = player_movement_logic(
                &tile_storage,
                &mut player_transform.translation,
                &mut player_coords,
                &event.direction,
            );
            if did_move && maybe_primary_player.is_some() {
                move_neighboring_player_events.send(MoveNeighboringPlayersEvent {
                    grid_coords: level_pos.0,
                    direction: event.direction,
                })
            }
        }
    }
}

pub fn move_neighboring_players(
    mut move_neighboring_player_events: EventReader<MoveNeighboringPlayersEvent>,
    mut player_query: Query<(&mut GridCoords, &mut Transform), With<Player>>,
    level_query: Query<(&Children, &LevelPosition), With<Handle<LdtkLevel>>>,
    layer_query: Query<(&LayerMetadata, &TileStorage)>,
) {
    for event in move_neighboring_player_events.iter() {
        for (level_children, _) in level_query
            .iter()
            .filter(|(_, level_pos)| level_pos.0.is_neighbor(event.grid_coords))
        {
            for &child in level_children {
                let Ok((metadata, tile_storage)) = layer_query.get(child) else { continue };
                if metadata.identifier != "Tiles" {
                    continue;
                }
                for &child in level_children {
                    let Ok((mut player_coords, mut player_transform)) = player_query.get_mut(child) else { continue };
                    let _did_move = player_movement_logic(
                        &tile_storage,
                        &mut player_transform.translation,
                        &mut player_coords,
                        &event.direction,
                    );
                }
            }
        }
    }
}

fn player_movement_logic(
    tile_storage: &TileStorage,
    player_translation: &mut Vec3,
    player_coords: &mut GridCoords,
    direction: &Direction,
) -> bool {
    let new_coords = *player_coords + direction.unit_grid_coords();
    if let Some(_tile_entity) =
        tile_storage.get(&TilePos::new(new_coords.x as u32, new_coords.y as u32))
    {
        // TODO: more logic
        *player_translation += direction.unit_vec().extend(0.) * 32.;
        *player_coords = new_coords;
        true
    } else {
        false
    }
}
