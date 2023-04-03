use crate::{loading::GameAssets, player::Player, GameState};
use bevy::{prelude::*, utils::HashMap};
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
                    setup_ldtk_levels_on_spawn.run_if(resource_exists::<ActiveLevel>()),
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
pub struct ActiveLevel {
    pub grid_size: usize,
    pub grid_width: i32,
    pub grid_height: i32,
    pub item_width_px: i32,
    pub item_height_px: i32,
    pub initial_placement: HashMap<MetaGridPos, String>,
    pub active_placement: HashMap<MetaGridPos, Entity>,
}

impl ActiveLevel {
    fn get_translation(&self, grid_pos: MetaGridPos) -> Vec2 {
        let col_offset =
            (grid_pos.col as f32 - 0.5 * (self.grid_width as f32 - 1.)) * self.item_width_px as f32;
        // row offset is flipped
        let row_offset = -(grid_pos.row as f32 - 0.5 * (self.grid_height as f32 - 1.))
            * self.item_height_px as f32;
        // levels are loaded with the bottom left corner at the world origin, so
        // we offset the level so that the center of the level aligns with the
        // world origin.
        let center_offset = Vec2::new(-self.item_width_px as f32, -self.item_height_px as f32) / 2.;
        Vec2::new(col_offset, row_offset) + center_offset
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MetaGridPos {
    pub row: i32,
    pub col: i32,
}

impl MetaGridPos {
    pub fn new(row: i32, col: i32) -> Self {
        Self { row, col }
    }

    pub fn is_neighbor(&self, other: Self) -> bool {
        (self.row - other.row).abs() + (self.col - other.col).abs() == 1
    }

    pub fn neighbors(&self) -> [Self; 8] {
        [
            Self::new(self.row - 1, self.col - 1),
            Self::new(self.row - 1, self.col),
            Self::new(self.row - 1, self.col + 1),
            Self::new(self.row, self.col - 1),
            Self::new(self.row, self.col + 1),
            Self::new(self.row + 1, self.col - 1),
            Self::new(self.row + 1, self.col),
            Self::new(self.row + 1, self.col + 1),
        ]
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

#[derive(Component)]
pub struct LevelPosition(pub MetaGridPos);

// ================
// ==== EVENTS ====
// ================

struct LoadLevelEvent {
    level_num: i32,
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
    event_writer.send(LoadLevelEvent { level_num: 0 });
}

fn load_level(
    mut commands: Commands,
    ldtk_assets: Res<Assets<LdtkAsset>>,
    mut ldtk_world_query: Query<(&mut LevelSet, &Handle<LdtkAsset>)>,
    mut event_reader: EventReader<LoadLevelEvent>,
) {
    if let Some(event) = event_reader.iter().next() {
        let (mut level_set, ldtk_handle) = ldtk_world_query.single_mut();
        let ldtk_asset = ldtk_assets.get(&ldtk_handle).expect("ldtk asset exists");
        // these are updated as we iterate over the levels
        let mut grid_width = 1;
        let mut grid_height = 1;
        let mut item_width_px = 0;
        let mut item_height_px = 0;
        let mut initial_placement = HashMap::new();
        ldtk_asset
            .iter_levels()
            .filter_map(|level| {
                level
                    .field_instances
                    .iter()
                    .any(|field| {
                        field.identifier == "LevelNum"
                            && matches!(field.value, FieldValue::Int(Some(num)) if num == event.level_num)
                    }).then_some(level)
            }).for_each(|level| {
                let row = level.field_instances.iter().filter(|field| field.identifier == "GridRow").find_map(|field| {
                    if let FieldValue::Int(Some(row)) = field.value {
                        Some(row)
                    } else {
                        None
                    }
                }).expect("GridRow field is defined");
                let col = level.field_instances.iter().filter(|field| field.identifier == "GridCol").find_map(|field| {
                    if let FieldValue::Int(Some(col)) = field.value {
                        Some(col)
                    } else {
                        None
                    }
                }).expect("GridCol field is defined");
                grid_height = grid_height.max(row + 1);
                grid_width = grid_width.max(col + 1);
                item_width_px = item_width_px.max(level.px_wid);
                item_height_px = item_height_px.max(level.px_hei);
                initial_placement.insert(MetaGridPos::new(row, col), level.iid.clone());
            });
        level_set.iids = initial_placement.values().cloned().collect();
        let grid_size = (grid_width * grid_height) as usize;
        commands.insert_resource(ActiveLevel {
            grid_size,
            grid_width,
            grid_height,
            item_width_px,
            item_height_px,
            initial_placement,
            // this is initialized as the LDtk levels are spawned
            active_placement: HashMap::new(),
        });
    }
    event_reader.clear();
}

fn setup_ldtk_levels_on_spawn(
    mut commands: Commands,
    mut active_level: ResMut<ActiveLevel>,
    ldtk_level_assets: Res<Assets<LdtkLevel>>,
    mut ldtk_level_query: Query<
        (Entity, &Handle<LdtkLevel>, &mut Transform),
        Added<Handle<LdtkLevel>>,
    >,
) {
    for (level_entity, level_handle, mut level_transform) in &mut ldtk_level_query {
        let ldtk_level = ldtk_level_assets
            .get(&level_handle)
            .expect("ldtk level is loaded");
        let (&grid_pos, _) = active_level
            .initial_placement
            .iter()
            .find(|(_pos, iid)| **iid == ldtk_level.level.iid)
            .expect("level iid exists in active level");
        active_level.active_placement.insert(grid_pos, level_entity);
        commands
            .entity(level_entity)
            .insert(LevelPosition(grid_pos));
        level_transform.translation = active_level.get_translation(grid_pos).extend(0.);
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
        event_writer.send(LoadLevelEvent { level_num: 0 })
    }
}
