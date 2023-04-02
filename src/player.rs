use crate::GameState;
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;
use leafwing_input_manager::prelude::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(InputManagerPlugin::<PlayerAction>::default())
            .register_ldtk_entity::<PlayerBundle>("Player")
            .add_systems(
                (add_components_to_primary_player, player_movement)
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

// #[derive(Clone, Copy, PartialEq, Eq)]
// enum Direction {
//     Up,
//     Down,
//     Left,
//     Right,
// }

// ================
// ==== EVENTS ====
// ================

// struct MovePlayerEvent {
//     player: Entity,
//     direction: Direction,
// }

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
    #[sprite_sheet_bundle]
    #[bundle]
    sprite_sheet: SpriteSheetBundle,
    #[with(player_physics)]
    #[bundle]
    physics: (RigidBody, Collider, ActiveEvents),
    #[from_entity_instance]
    entity_instance: EntityInstance,
}

fn player_physics(_: &EntityInstance) -> (RigidBody, Collider, ActiveEvents) {
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

fn player_movement(
    mut player_query: Query<(&ActionState<PlayerAction>, &mut Transform), With<Player>>,
) {
    for (action_state, mut transform) in &mut player_query {
        if action_state.just_pressed(PlayerAction::MoveUp) {
            transform.translation += Vec3::Y * 32.;
        }
        if action_state.just_pressed(PlayerAction::MoveDown) {
            transform.translation -= Vec3::Y * 32.;
        }
        if action_state.just_pressed(PlayerAction::MoveRight) {
            transform.translation += Vec3::X * 32.;
        }
        if action_state.just_pressed(PlayerAction::MoveLeft) {
            transform.translation -= Vec3::X * 32.;
        }
    }
}
