use crate::{loading::GameAssets, player::Player, GameState};
use bevy::{prelude::*, utils::HashMap};
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

const LEVEL_SPAWN_DELAY_SEC: f32 = 0.5;

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveLevel>()
            .add_event::<LoadLevelEvent>()
            .register_ldtk_int_cell::<FloorBundle>(1)
            .register_ldtk_int_cell::<GoalBundle>(2)
            .register_ldtk_int_cell::<WallBundle>(3)
            .add_system(setup.in_schedule(OnEnter(GameState::InGame)))
            .add_systems(
                (
                    load_level,
                    setup_ldtk_levels_on_spawn,
                    add_goal_sensor,
                    load_next_level_on_goal,
                    debug,
                    level_countdown_timer.run_if(resource_exists::<LevelSpawnCountdown>()),
                )
                    .in_set(OnUpdate(GameState::InGame)),
            );
    }
}

// ===================
// ==== RESOURCES ====
// ===================

#[derive(Resource, Default)]
pub struct ActiveLevel {
    pub level_num: i32,
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

#[derive(Resource)]
struct LevelSpawnCountdown {
    timer: Timer,
    level_num: i32,
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
}

// ====================
// ==== COMPONENTS ====
// ====================

#[derive(Component)]
pub enum TileType {
    Floor,
    Goal,
    Wall,
}

impl From<IntGridCell> for TileType {
    fn from(int_grid_cell: IntGridCell) -> Self {
        match int_grid_cell.value {
            1 => Self::Floor,
            2 => Self::Goal,
            3 => Self::Wall,
            _ => panic!("unknown tile type"),
        }
    }
}

#[derive(Bundle, LdtkIntCell)]
struct GameTileBundle {
    #[from_int_grid_cell]
    tile_type: TileType,
}

#[derive(Component, Default)]
pub struct Floor;

#[derive(Bundle, LdtkIntCell)]
pub struct FloorBundle {
    floor: Floor,
    #[from_int_grid_cell]
    tile_type: TileType,
}

#[derive(Component, Default)]
pub struct Goal;

#[derive(Bundle, LdtkIntCell)]
pub struct GoalBundle {
    goal: Goal,
    #[from_int_grid_cell]
    tile_type: TileType,
}

#[derive(Component, Default)]
pub struct Wall;

#[derive(Bundle, LdtkIntCell)]
pub struct WallBundle {
    wall: Wall,
    #[from_int_grid_cell]
    tile_type: TileType,
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
            // only include the levels with the correct LevelNum
            .filter(|level| {
                level
                    .field_instances
                    .iter()
                    .any(|field| {
                        field.identifier == "LevelNum"
                            && matches!(field.value, FieldValue::Int(Some(num)) if num == event.level_num)
                    })
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
        commands.insert_resource(ActiveLevel {
            level_num: event.level_num,
            grid_width,
            grid_height,
            item_width_px,
            item_height_px,
            initial_placement,
            // this is filled with entities as the LDtk levels are spawned
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

fn load_next_level_on_goal(
    mut commands: Commands,
    active_level: Res<ActiveLevel>,
    level_spawn_countdown: Option<Res<LevelSpawnCountdown>>,
    player_query: Query<&Player>,
    goal_query: Query<&Goal>,
    mut collision_events: EventReader<CollisionEvent>,
    // mut event_writer: EventWriter<LoadLevelEvent>,
) {
    // only continue if we're not already waiting to load a new level
    if level_spawn_countdown.is_none() {
        for event in collision_events.iter() {
            match *event {
                CollisionEvent::Started(a, b, _) => {
                    if player_query.contains(a) && goal_query.contains(b)
                        || player_query.contains(b) && goal_query.contains(a)
                    {
                        // event_writer.send(LoadLevelEvent {
                        //     level_num: active_level.level_num + 1,
                        // });
                        commands.insert_resource(LevelSpawnCountdown {
                            timer: Timer::from_seconds(LEVEL_SPAWN_DELAY_SEC, TimerMode::Once),
                            level_num: active_level.level_num + 1,
                        });
                    }
                }
                CollisionEvent::Stopped(_, _, _) => {}
            }
        }
    }
}

fn level_countdown_timer(
    mut commands: Commands,
    time: Res<Time>,
    mut level_spawn_countdown: ResMut<LevelSpawnCountdown>,
    mut load_level_events: EventWriter<LoadLevelEvent>,
) {
    if level_spawn_countdown
        .timer
        .tick(time.delta())
        .just_finished()
    {
        commands.remove_resource::<LevelSpawnCountdown>();
        load_level_events.send(LoadLevelEvent {
            level_num: level_spawn_countdown.level_num,
        })
    }
}

fn debug(input: Res<Input<KeyCode>>, mut event_writer: EventWriter<LoadLevelEvent>) {
    if input.just_pressed(KeyCode::Space) {
        event_writer.send(LoadLevelEvent { level_num: 0 })
    }
}
