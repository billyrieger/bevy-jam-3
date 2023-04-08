use crate::{
    boundary::{BoundaryEdge, BoundaryPlugin},
    loading::GameAssets,
    player::Player,
    GameState,
};
use bevy::{prelude::*, render::view::RenderLayers, utils::HashMap};
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
            .register_ldtk_int_cell::<BoundaryBundle>(4)
            .add_plugin(BoundaryPlugin)
            .add_system(setup.in_schedule(OnEnter(GameState::InGame)))
            .add_systems(
                (load_level, setup_ldtk_levels_on_spawn, add_goal_sensors)
                    .in_set(OnUpdate(GameState::InGame)),
            )
            .add_systems(
                (
                    update_goal_tile_status,
                    check_all_goal_tiles.run_if(any_with_component::<Goal>()),
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
    pub item_grid_width: i32,
    pub item_grid_height: i32,
    pub item_width_px: i32,
    pub item_height_px: i32,
    pub initial_placement: HashMap<MetaGridPos, String>,
}

impl ActiveLevel {
    pub fn boundary_coords(
        &self,
        boundary_edge: BoundaryEdge,
    ) -> Box<dyn Iterator<Item = GridCoords>> {
        let item_grid_width = self.item_grid_width;
        let item_grid_height = self.item_grid_height;
        // arrows are along the edges of the level but NOT at the corners, so skip the first and last indices.
        // "normally" the range would be 0..width. instead we do 1..(width - 1).
        match boundary_edge {
            BoundaryEdge::Top => Box::new(
                (1..(item_grid_width - 1)).map(move |x| GridCoords::new(x, item_grid_height - 1)),
            ),
            BoundaryEdge::Bottom => {
                Box::new((1..(item_grid_width - 1)).map(move |x| GridCoords::new(x, 0)))
            }
            BoundaryEdge::Left => {
                Box::new((1..(self.item_grid_height - 1)).map(move |y| GridCoords::new(0, y)))
            }
            BoundaryEdge::Right => Box::new(
                (1..(item_grid_height - 1)).map(move |y| GridCoords::new(item_grid_width - 1, y)),
            ),
        }
    }

    pub fn grid_coords_to_center_translation(&self, grid_coords: GridCoords) -> Vec3 {
        let _center_offset = Vec2::new(-self.item_width_px as f32, self.item_height_px as f32) / 2.;
        let _x = grid_coords.x * crate::GRID_SIZE - self.item_width_px / 2;
        let _y = grid_coords.y * crate::GRID_SIZE - self.item_height_px / 2;
        todo!()
    }

    pub fn total_width_px(&self) -> i32 {
        self.grid_width * self.item_width_px
    }

    pub fn total_height_px(&self) -> i32 {
        self.grid_height * self.item_height_px
    }

    pub fn unpadded_item_width_px(&self) -> i32 {
        self.item_width_px - 2 * crate::GRID_SIZE
    }

    pub fn unpadded_item_height_px(&self) -> i32 {
        self.item_height_px - 2 * crate::GRID_SIZE
    }

    pub fn get_center_translation_for_texture(&self, grid_pos: MetaGridPos) -> Vec2 {
        let col_offset =
            (grid_pos.col as f32 - 0.5 * (self.grid_width as f32 - 1.)) * self.item_width_px as f32;
        let row_offset = (grid_pos.row as f32 - 0.5 * (self.grid_height as f32 - 1.))
            * self.item_height_px as f32;
        Vec2::new(crate::WIDTH as f32, crate::HEIGHT as f32) / 2.
            + Vec2::new(col_offset, row_offset)
    }

    pub fn get_translation(&self, grid_pos: MetaGridPos) -> Vec2 {
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
pub struct LevelSpawnCountdown {
    pub timer: Timer,
    pub level_num: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
pub struct LevelPosition(pub MetaGridPos);

#[derive(Component)]
pub enum TileType {
    Floor,
    Goal,
    Wall,
    Boundary,
}

impl From<IntGridCell> for TileType {
    fn from(int_grid_cell: IntGridCell) -> Self {
        match int_grid_cell.value {
            1 => Self::Floor,
            2 => Self::Goal,
            3 => Self::Wall,
            4 => Self::Boundary,
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
pub struct Goal {
    activated: bool,
}

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

#[derive(Component, Default)]
pub struct Boundary;

#[derive(Bundle, LdtkIntCell)]
pub struct BoundaryBundle {
    boundary: Boundary,
    #[from_int_grid_cell]
    tile_type: TileType,
}

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
    commands
        .spawn(LdtkWorldBundle {
            ldtk_handle: game_assets.levels.clone(),
            level_set: LevelSet::default(),
            ..default()
        })
        .insert(RenderLayers::layer(1));
    event_writer.send(LoadLevelEvent { level_num: 0 });
}

fn load_level(
    mut commands: Commands,
    ldtk_assets: Res<Assets<LdtkAsset>>,
    mut ldtk_world_query: Query<(&mut LevelSet, &Handle<LdtkAsset>)>,
    mut event_reader: EventReader<LoadLevelEvent>,
) {
    if let Some(event) = event_reader.iter().next() {
        commands.remove_resource::<LevelSpawnCountdown>();

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
            item_grid_width: item_width_px / crate::GRID_SIZE,
            item_grid_height: item_height_px / crate::GRID_SIZE,
            item_width_px,
            item_height_px,
            initial_placement,
        });
    }
    event_reader.clear();
}

fn setup_ldtk_levels_on_spawn(
    mut commands: Commands,
    active_level: ResMut<ActiveLevel>,
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
        commands
            .entity(level_entity)
            .insert(LevelPosition(grid_pos));
        level_transform.translation = active_level.get_translation(grid_pos).extend(0.);
    }
}

fn add_goal_sensors(mut commands: Commands, goal_query: Query<Entity, Added<Goal>>) {
    for entity in &goal_query {
        commands.entity(entity).insert((
            Collider::cuboid(8., 8.),
            Sensor,
            ActiveEvents::COLLISION_EVENTS,
        ));
    }
}

fn update_goal_tile_status(
    player_query: Query<&Player>,
    mut goal_query: Query<&mut Goal>,
    mut collision_events: EventReader<CollisionEvent>,
) {
    for event in collision_events.iter() {
        let event_started = matches!(event, CollisionEvent::Started(_, _, _));
        match *event {
            CollisionEvent::Started(a, b, _) | CollisionEvent::Stopped(a, b, _) => {
                if player_query.contains(a) && goal_query.contains(b)
                    || player_query.contains(b) && goal_query.contains(a)
                {
                    let mut goal = if goal_query.contains(a) {
                        goal_query.get_mut(a).unwrap()
                    } else {
                        goal_query.get_mut(b).unwrap()
                    };
                    goal.activated = event_started;
                }
            }
        }
    }
}

fn check_all_goal_tiles(
    mut commands: Commands,
    active_level: Res<ActiveLevel>,
    level_spawn_countdown: Option<Res<LevelSpawnCountdown>>,
    goal_query: Query<&Goal>,
) {
    // only continue if we're not already waiting to load a new level
    if level_spawn_countdown.is_some() {
        return;
    }
    if goal_query.iter().all(|goal| goal.activated) {
        commands.insert_resource(LevelSpawnCountdown {
            timer: Timer::from_seconds(LEVEL_SPAWN_DELAY_SEC, TimerMode::Once),
            level_num: active_level.level_num + 1,
        });
    }
}

fn level_countdown_timer(
    time: Res<Time>,
    mut level_spawn_countdown: ResMut<LevelSpawnCountdown>,
    mut load_level_events: EventWriter<LoadLevelEvent>,
) {
    if level_spawn_countdown
        .timer
        .tick(time.delta())
        .just_finished()
    {
        load_level_events.send(LoadLevelEvent {
            level_num: level_spawn_countdown.level_num,
        })
    }
}
