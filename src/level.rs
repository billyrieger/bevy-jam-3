use crate::{
    boundary::BoundaryPlugin,
    loading::GameAssets,
    player::{Player, PrimaryPlayer, QueuedInput},
    GameState, GRID_SIZE, STARTING_LEVEL, ui::DragAreaPosition,
};
use bevy::{prelude::*, render::view::RenderLayers, utils::HashMap};
use bevy_ecs_ldtk::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use bevy_particle_systems::*;

const LEVEL_SPAWN_DELAY_SEC: f32 = 1.;
const ACTIVE_LEVEL_COLOR: Color = Color::rgb(1., 1., 1.);
const INACTIVE_LEVEL_COLOR: Color = Color::rgb(0.5, 0.5, 0.5);

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<LoadLevelEvent>()
            .add_event::<ReloadLevelEvent>()
            .register_ldtk_int_cell::<FloorBundle>(1)
            .register_ldtk_int_cell::<GoalBundle>(2)
            .register_ldtk_int_cell::<WallBundle>(3)
            .register_ldtk_int_cell::<BoundaryBundle>(4)
            .add_plugin(BoundaryPlugin)
            .add_systems((setup, prepare_level_data).in_schedule(OnEnter(GameState::InGame)))
            .add_systems(
                (
                    load_level,
                    add_particles_to_goals.run_if(resource_exists::<CurrentMetaLevel>()),
                    move_particles_up,
                    reload_level.run_if(resource_exists::<CurrentMetaLevel>()),
                    setup_ldtk_levels_on_spawn.run_if(resource_exists::<CurrentMetaLevel>()),
                    darken_inactive_levels,
                )
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MetaGridCoords {
    pub row: i32,
    pub col: i32,
}

impl MetaGridCoords {
    pub fn new(row: i32, col: i32) -> Self {
        Self { row, col }
    }

    pub fn is_neighbor(&self, other: Self) -> bool {
        (self.row - other.row).abs() + (self.col - other.col).abs() == 1
    }
}

#[derive(Clone, Debug)]
pub struct MetaLevel {
    pub level_num: i32,
    pub meta_grid_width: i32,
    pub meta_grid_height: i32,
    pub level_grid_width: i32,
    pub level_grid_height: i32,
    pub initial_placement: HashMap<MetaGridCoords, String>,
}

impl MetaLevel {
    pub fn level_width_px(&self) -> i32 {
        self.level_grid_width * GRID_SIZE
    }

    pub fn level_height_px(&self) -> i32 {
        self.level_grid_height * GRID_SIZE
    }

    pub fn total_width_px(&self) -> i32 {
        self.meta_grid_width * self.level_grid_width * GRID_SIZE
    }

    pub fn total_height_px(&self) -> i32 {
        self.meta_grid_height * self.level_grid_height * GRID_SIZE
    }

    pub fn unpadded_item_width_px(&self) -> i32 {
        (self.level_grid_width - 1) * GRID_SIZE
    }

    pub fn unpadded_item_height_px(&self) -> i32 {
        (self.level_grid_height - 1) * GRID_SIZE
    }

    pub fn get_center_translation_for_texture(&self, meta_coords: MetaGridCoords) -> Vec2 {
        let col_offset = (meta_coords.col as f32 - 0.5 * (self.meta_grid_width as f32 - 1.))
            * self.level_width_px() as f32;
        let row_offset = (meta_coords.row as f32 - 0.5 * (self.meta_grid_height as f32 - 1.))
            * self.level_height_px() as f32;
        Vec2::new(crate::WIDTH as f32, crate::HEIGHT as f32) / 2.
            + Vec2::new(col_offset, row_offset)
    }

    pub fn get_translation(&self, grid_pos: MetaGridCoords) -> Vec2 {
        let col_offset = (grid_pos.col as f32 - 0.5 * (self.meta_grid_width as f32 - 1.))
            * self.level_width_px() as f32;
        // row offset is flipped
        let row_offset = -(grid_pos.row as f32 - 0.5 * (self.meta_grid_height as f32 - 1.))
            * self.level_height_px() as f32;
        // levels are loaded with the bottom left corner at the world origin, so
        // we offset the level so that the center of the level aligns with the
        // world origin.
        let center_offset = Vec2::new(self.level_grid_width as f32, self.level_grid_height as f32)
            * GRID_SIZE as f32
            / 2.;
        Vec2::new(col_offset, row_offset) - center_offset
    }

    pub fn top_boundary_coords(&self) -> impl Iterator<Item = GridCoords> + '_ {
        (1..(self.level_grid_width - 1)).map(|x| GridCoords::new(x, self.level_grid_height - 1))
    }

    pub fn bottom_boundary_coords(&self) -> impl Iterator<Item = GridCoords> + '_ {
        (1..(self.level_grid_width - 1)).map(|x| GridCoords::new(x, 0))
    }

    pub fn left_boundary_coords(&self) -> impl Iterator<Item = GridCoords> + '_ {
        (1..(self.level_grid_height - 1)).map(|y| GridCoords::new(0, y))
    }

    pub fn right_boundary_coords(&self) -> impl Iterator<Item = GridCoords> + '_ {
        (1..(self.level_grid_height - 1))
            .map(move |y| GridCoords::new(self.level_grid_width - 1, y))
    }

    pub fn grid_coords_to_translation(&self, grid_coords: GridCoords) -> Vec2 {
        let x = grid_coords.x * GRID_SIZE;
        let y = grid_coords.y * GRID_SIZE;
        Vec2::new(x as f32, y as f32)
    }
}

// ===================
// ==== RESOURCES ====
// ===================

#[derive(Resource)]
pub struct AllMetaLevels(Vec<MetaLevel>);

#[derive(Resource)]
pub struct CurrentMetaLevel(pub MetaLevel);

#[derive(Resource)]
pub struct LevelSpawnCountdown {
    pub timer: Timer,
    pub level_num: i32,
}

// ================
// ==== EVENTS ====
// ================

pub struct LoadLevelEvent {
    pub level_num: i32,
}

pub struct ReloadLevelEvent;

// ====================
// ==== COMPONENTS ====
// ====================

#[derive(Component)]
pub struct LevelPosition(pub MetaGridCoords);

#[derive(Component)]
pub struct IsActive(pub bool);

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
    pub tile_type: TileType,
}

#[derive(Component, Default)]
pub struct Floor;

#[derive(Bundle, LdtkIntCell)]
pub struct FloorBundle {
    pub floor: Floor,
    #[from_int_grid_cell]
    pub tile_type: TileType,
}

#[derive(Component, Default)]
pub struct Goal {
    pub activated: bool,
}

#[derive(Bundle, LdtkIntCell)]
pub struct GoalBundle {
    pub goal: Goal,
    #[from_int_grid_cell]
    pub tile_type: TileType,
}

#[derive(Component)]
pub struct GoalParticles;

#[derive(Component, Default)]
pub struct Wall;

#[derive(Bundle, LdtkIntCell)]
pub struct WallBundle {
    pub wall: Wall,
    #[from_int_grid_cell]
    pub tile_type: TileType,
}

#[derive(Component, Default)]
pub struct Boundary;

#[derive(Bundle, LdtkIntCell)]
pub struct BoundaryBundle {
    pub boundary: Boundary,
    #[from_int_grid_cell]
    pub tile_type: TileType,
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
    event_writer.send(LoadLevelEvent {
        level_num: STARTING_LEVEL,
    });
}

fn prepare_level_data(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    ldtk_assets: Res<Assets<LdtkAsset>>,
) {
    let mut all_levels = AllMetaLevels(vec![]);
    let ldtk_asset = ldtk_assets
        .get(&game_assets.levels)
        .expect("LDtk asset exists");
    for level_num in 0.. {
        // these are updated as we iterate over the levels
        let mut meta_grid_width = 1;
        let mut meta_grid_height = 1;
        let mut level_grid_width = 0;
        let mut level_grid_height = 0;
        let mut initial_placement = HashMap::new();

        for level in ldtk_asset
            .iter_levels()
            // only include the levels with the correct LevelNum
            .filter(|level| {
                level.field_instances.iter().any(|field| {
                    field.identifier == "LevelNum"
                        && matches!(field.value, FieldValue::Int(Some(num)) if num == level_num)
                })
            })
        {
            let row = level
                .field_instances
                .iter()
                .find_map(|field| match (&field.identifier, &field.value) {
                    (ident, FieldValue::Int(Some(val))) if ident == "GridRow" => Some(*val),
                    _ => None,
                })
                .expect("GridRow field is defined");
            let col = level
                .field_instances
                .iter()
                .find_map(|field| match (&field.identifier, &field.value) {
                    (ident, FieldValue::Int(Some(val))) if ident == "GridCol" => Some(*val),
                    _ => None,
                })
                .expect("GridRow field is defined");
            meta_grid_height = meta_grid_height.max(row + 1);
            meta_grid_width = meta_grid_width.max(col + 1);
            level_grid_width = level_grid_width.max(level.px_wid / GRID_SIZE);
            level_grid_height = level_grid_height.max(level.px_hei / GRID_SIZE);
            initial_placement.insert(MetaGridCoords::new(row, col), level.iid.clone());
        }

        if initial_placement.is_empty() {
            break;
        }

        all_levels.0.push(MetaLevel {
            level_num,
            meta_grid_width,
            meta_grid_height,
            level_grid_width,
            level_grid_height,
            initial_placement,
        });
    }

    commands.insert_resource(all_levels);
}

fn add_particles_to_goals(
    current_level: Res<CurrentMetaLevel>,
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    goals: Query<(&GridCoords, &Parent), Added<Goal>>,
) {
    for (&goal_coords, parent) in &goals {
        commands.entity(parent.get()).with_children(|parent| {
            parent.spawn(GoalParticles).insert(ParticleSystemBundle {
                particle_system: ParticleSystem {
                    max_particles: 100,
                    texture: ParticleTexture::Sprite(game_assets.pixel.clone()),
                    initial_speed: JitteredValue::jittered(500.0, -300.0..300.0),
                    velocity_modifiers: vec![VelocityModifier::Drag(0.05.into())],
                    lifetime: JitteredValue::jittered(0.5, -0.25..0.25),
                    color: ColorOverTime::Gradient(Curve::new(vec![
                        CurvePoint::new(Color::rgb(0., 1., 0.), 0.0),
                        CurvePoint::new(Color::rgba(0., 1., 0., 0.), 1.0),
                    ])),
                    spawn_rate_per_second: ValueOverTime::Constant(0.),
                    despawn_particles_with_system: true,
                    space: ParticleSpace::World,
                    looping: false,
                    system_duration_seconds: 2.,
                    max_distance: Some(500.),
                    scale: 2.0.into(),
                    bursts: vec![ParticleBurst::new(0.0, 100)],
                    ..ParticleSystem::default()
                },
                transform: Transform::from_translation(
                    current_level
                        .0
                        .grid_coords_to_translation(goal_coords)
                        .extend(0.),
                ),
                ..ParticleSystemBundle::default()
            });
        });
    }
}

fn load_level(
    mut commands: Commands,
    all_levels: Res<AllMetaLevels>,
    mut ldtk_world_query: Query<&mut LevelSet>,
    mut event_reader: EventReader<LoadLevelEvent>,
    mut queued_input: ResMut<QueuedInput>,
) {
    if let Some(event) = event_reader.iter().next() {
        commands.remove_resource::<LevelSpawnCountdown>();
        queued_input.0.clear();

        let mut level_set = ldtk_world_query.single_mut();
        let meta_level = all_levels
            .0
            .get(event.level_num as usize)
            .unwrap_or_else(|| all_levels.0.first().unwrap());

        level_set.iids = meta_level.initial_placement.values().cloned().collect();
        commands.insert_resource(CurrentMetaLevel(meta_level.clone()));
    }
    event_reader.clear();
}

fn reload_level(
    mut commands: Commands,
    current_level: Res<CurrentMetaLevel>,
    mut ldtk_world_query: Query<&mut LevelSet>,
    mut event_reader: EventReader<ReloadLevelEvent>,
    mut load_events: EventWriter<LoadLevelEvent>,
) {
    if let Some(_) = event_reader.iter().next() {
        let current_level_num = current_level.0.level_num;
        commands.remove_resource::<LevelSpawnCountdown>();
        let mut level_set = ldtk_world_query.single_mut();
        level_set.iids.clear();
        commands.remove_resource::<CurrentMetaLevel>();
        load_events.send(LoadLevelEvent {
            level_num: current_level_num,
        });
    }
    event_reader.clear();
}

fn setup_ldtk_levels_on_spawn(
    mut commands: Commands,
    current_level: Res<CurrentMetaLevel>,
    ldtk_level_assets: Res<Assets<LdtkLevel>>,
    mut ldtk_level_query: Query<
        (Entity, &Children, &Handle<LdtkLevel>, &mut Transform),
        Added<Handle<LdtkLevel>>,
    >,
    primary_players: Query<&PrimaryPlayer>,
) {
    for (level_entity, level_children, level_handle, mut level_transform) in &mut ldtk_level_query {
        let ldtk_level = ldtk_level_assets
            .get(&level_handle)
            .expect("ldtk level is loaded");
        let (&grid_pos, _) = current_level
            .0
            .initial_placement
            .iter()
            .find(|(_pos, iid)| **iid == ldtk_level.level.iid)
            .expect("level iid exists in active level");
        let is_active = level_children
            .iter()
            .any(|child| primary_players.contains(*child));
        commands
            .entity(level_entity)
            .insert(LevelPosition(grid_pos))
            .insert(IsActive(is_active));
        level_transform.translation = current_level.0.get_translation(grid_pos).extend(0.);
    }
}

fn update_goal_tile_status(
    mut goals: Query<(&Parent, &GridCoords, &mut Goal)>,
    players: Query<(&Parent, &GridCoords), With<Player>>,
    layers: Query<&Parent, With<LayerMetadata>>,
) {
    for (goal_parent, goal_coords, mut goal) in &mut goals {
        let layer_parent = layers
            .get(goal_parent.get())
            .expect("goal's parent is a layer");
        let goal_level = layer_parent.get();
        goal.activated = players.iter().any(|(player_parent, player_coords)| {
            player_parent.get() == goal_level && player_coords == goal_coords
        });
    }
}

fn check_all_goal_tiles(
    mut commands: Commands,
    current_level: Res<CurrentMetaLevel>,
    level_spawn_countdown: Option<Res<LevelSpawnCountdown>>,
    goal_query: Query<&Goal>,
    goal_particles: Query<Entity, With<GoalParticles>>,
) {
    // only continue if we're not already waiting to load a new level
    if level_spawn_countdown.is_some() {
        return;
    }
    if goal_query.iter().all(|goal| goal.activated) {
        println!("done!!!");
        commands.insert_resource(LevelSpawnCountdown {
            timer: Timer::from_seconds(LEVEL_SPAWN_DELAY_SEC, TimerMode::Once),
            level_num: current_level.0.level_num + 1,
        });
        for goal_particles in &goal_particles {
            commands.entity(goal_particles).insert(Playing);
        }
    }
}

fn level_countdown_timer(
    time: Res<Time>,
    mut countdown: ResMut<LevelSpawnCountdown>,
    mut load_level_events: EventWriter<LoadLevelEvent>,
) {
    if countdown.timer.tick(time.delta()).just_finished() {
        load_level_events.send(LoadLevelEvent {
            level_num: countdown.level_num,
        })
    }
}

fn darken_inactive_levels(
    levels: Query<(&Children, &IsActive), Changed<IsActive>>,
    mut drag_areas: Query<(&DragAreaPosition, &mut BackgroundColor)>,
    layers: Query<(&LayerMetadata, &TileStorage)>,
    primary_players: Query<&PrimaryPlayer>,
    mut tiles: Query<&mut TileColor>,
    mut sprites: Query<&mut TextureAtlasSprite>,
) {
    for (level_children, level_is_active) in levels.iter().filter(|(children, _)| {
        children
            .iter()
            .all(|child| !primary_players.contains(*child))
    }) {
        let color = if level_is_active.0 {
            ACTIVE_LEVEL_COLOR
        } else {
            INACTIVE_LEVEL_COLOR
        };
        let (_, tile_storage) = level_children
            .iter()
            .filter_map(|child| layers.get(*child).ok())
            .find(|(metadata, _)| metadata.identifier == "Tiles")
            .expect("Tiles layer exists");
        for tile in tile_storage.iter().filter_map(|x| *x) {
            let mut tile_color = tiles.get_mut(tile).expect("tile is in tile query");
            tile_color.0 = color;
        }

        for &child in level_children {
            if let Ok(mut sprite) = sprites.get_mut(child) {
                sprite.color = color;
            }
        }
    }
}

fn move_particles_up(mut particles: Query<&mut Transform, With<Particle>>) {
    for mut transform in &mut particles {
        transform.translation.z = 20.;
    }
}
