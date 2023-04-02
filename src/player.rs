use crate::GameState;
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use leafwing_input_manager::prelude::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(InputManagerPlugin::<PlayerAction>::default())
            .register_ldtk_entity::<PlayerBundle>("Player")
            .add_system(player_movement.in_set(OnUpdate(GameState::InGame)));
    }
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum PlayerAction {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
}

// ====================
// ==== COMPONENTS ====
// ====================

#[derive(Component, Default)]
pub struct Player;

#[derive(Bundle, LdtkEntity)]
pub struct PlayerBundle {
    player: Player,
    #[sprite_sheet_bundle]
    #[bundle]
    sprite_sheet: SpriteSheetBundle,
    #[with(player_input_manager)]
    #[bundle]
    input_manager: InputManagerBundle<PlayerAction>,
}

fn player_input_manager(_: &EntityInstance) -> InputManagerBundle<PlayerAction> {
    InputManagerBundle {
        action_state: ActionState::default(),
        input_map: InputMap::new([
            (KeyCode::Left, PlayerAction::MoveLeft),
            (KeyCode::Right, PlayerAction::MoveRight),
            (KeyCode::Up, PlayerAction::MoveUp),
            (KeyCode::Down, PlayerAction::MoveDown),
        ]),
        ..default()
    }
}

// =================
// ==== SYSTEMS ====
// =================

fn player_movement(
    mut player_query: Query<(&ActionState<PlayerAction>, &mut Transform), With<Player>>,
) {
    if let Ok((action_state, mut transform)) = player_query.get_single_mut() {
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
