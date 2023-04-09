use crate::{
    level::{CurrentMetaLevel, LevelPosition},
    player::PrimaryPlayer,
    util::grid_coords_to_tile_pos,
    GameState,
};
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_ecs_tilemap::prelude::*;

pub struct BoundaryPlugin;

impl Plugin for BoundaryPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            add_components_to_arrow_tiles
                .run_if(resource_exists::<CurrentMetaLevel>())
                .in_set(OnUpdate(GameState::InGame)),
        )
        .add_systems(
            (
                clear_boundary_arrows,
                update_boundary_arrows_pointing_from.run_if(any_with_component::<PrimaryPlayer>()),
                update_boundary_arrows_pointing_to.run_if(any_with_component::<PrimaryPlayer>()),
            )
                .in_set(OnUpdate(GameState::InGame)),
        );
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BoundaryEdge {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub enum ArrowDirection {
    Entering,
    Leaving,
}

// ====================
// ==== COMPONENTS ====
// ====================

#[derive(Component)]
struct BoundaryArrow {
    edge: BoundaryEdge,
    direction: ArrowDirection,
}

// =================
// ==== SYSTEMS ====
// =================

fn add_components_to_arrow_tiles(
    mut commands: Commands,
    current_level: Res<CurrentMetaLevel>,
    layers: Query<(&LayerMetadata, &TileStorage), Added<TileStorage>>,
) {
    // arrows are along the edges of the level but NOT at the corners, so skip the first and last indices.
    // "normally" the range would be 0..width. instead we do 1..(width - 1).
    let top_edge = (1..(current_level.0.level_grid_width - 1))
        .map(|x| GridCoords::new(x, current_level.0.level_grid_height - 1))
        .map(|coords| grid_coords_to_tile_pos(coords).unwrap())
        .map(|tile_pos| (tile_pos, BoundaryEdge::Top));
    let bottom_edge = (1..(current_level.0.level_grid_width - 1))
        .map(|x| GridCoords::new(x, 0))
        .map(|coords| grid_coords_to_tile_pos(coords).unwrap())
        .map(|tile_pos| (tile_pos, BoundaryEdge::Bottom));
    let left_edge = (1..(current_level.0.level_grid_height - 1))
        .map(|y| GridCoords::new(0, y))
        .map(|coords| grid_coords_to_tile_pos(coords).unwrap())
        .map(|tile_pos| (tile_pos, BoundaryEdge::Left));
    let right_edge = (1..(current_level.0.level_grid_height - 1))
        .map(|y| GridCoords::new(current_level.0.level_grid_width - 1, y))
        .map(|coords| grid_coords_to_tile_pos(coords).unwrap())
        .map(|tile_pos| (tile_pos, BoundaryEdge::Right));
    let edges = top_edge
        .chain(bottom_edge)
        .chain(left_edge)
        .chain(right_edge);

    for (metadata, tile_storage) in layers
        .iter()
        .filter(|(metadata, _)| ["ArrowsFrom", "ArrowsTo"].contains(&&*metadata.identifier))
    {
        let arrow_direction = if metadata.identifier == "ArrowsFrom" {
            ArrowDirection::Leaving
        } else {
            ArrowDirection::Entering
        };
        for (tile_pos, boundary_edge) in edges.clone() {
            let entity = tile_storage.get(&tile_pos).unwrap();
            commands.entity(entity).insert(BoundaryArrow {
                edge: boundary_edge,
                direction: arrow_direction,
            });
        }
    }
}

fn clear_boundary_arrows(
    layers: Query<(&LayerMetadata, &TileStorage)>,
    mut tiles: Query<&mut TileVisible>,
) {
    for (metadata, tile_storage) in &layers {
        if ["ArrowsFrom", "ArrowsTo"].contains(&&*metadata.identifier) {
            for tile_entity in tile_storage.iter().filter_map(|&t| t) {
                let mut visible = tiles.get_mut(tile_entity).expect("tile entity is a tile");
                visible.0 = false;
            }
        }
    }
}

fn update_boundary_arrows_pointing_from(
    current_level: Res<CurrentMetaLevel>,
    levels: Query<(&Children, &LevelPosition)>,
    layers: Query<(&LayerMetadata, &TileStorage)>,
    primary_players: Query<Entity, With<PrimaryPlayer>>,
    mut tiles: Query<&mut TileVisible>,
) {
    let (primary_level_children, primary_level_pos) = levels
        .iter()
        .find(|(children, _)| {
            children
                .iter()
                .any(|&child| primary_players.contains(child))
        })
        .expect("primary player exists in a level");

    let (_, arrows_tile_storage) = primary_level_children
        .iter()
        .filter_map(|&child| layers.get(child).ok())
        .find(|(metadata, _)| metadata.identifier == "ArrowsFrom")
        .expect("ArrowsFrom layer entity exists");

    let mut set_arrow_visible = |grid_coords: GridCoords| {
        let tile_pos = grid_coords_to_tile_pos(grid_coords).expect("edge coords are in bounds");
        let arrow_tile = arrows_tile_storage
            .get(&tile_pos)
            .expect("arrow tile exists at edge coords");
        let mut tile_visible = tiles
            .get_mut(arrow_tile)
            .expect("arrow tile matches tile query");
        tile_visible.0 = true;
    };

    // top edge
    if primary_level_pos.0.row > 0 {
        current_level
            .0
            .top_boundary_coords()
            .for_each(&mut set_arrow_visible);
    }
    // bottom edge
    if primary_level_pos.0.row < current_level.0.meta_grid_height - 1 {
        current_level
            .0
            .bottom_boundary_coords()
            .for_each(&mut set_arrow_visible);
    }
    // left edge
    if primary_level_pos.0.col > 0 {
        current_level
            .0
            .left_boundary_coords()
            .for_each(&mut set_arrow_visible);
    }
    // right edge
    if primary_level_pos.0.col < current_level.0.meta_grid_width - 1 {
        current_level
            .0
            .right_boundary_coords()
            .for_each(&mut set_arrow_visible);
    }
}

fn update_boundary_arrows_pointing_to(
    current_level: Res<CurrentMetaLevel>,
    levels: Query<(&Children, &LevelPosition)>,
    layers: Query<(&LayerMetadata, &TileStorage)>,
    primary_players: Query<Entity, With<PrimaryPlayer>>,
    mut tiles: Query<&mut TileVisible>,
) {
    let (_, primary_level_pos) = levels
        .iter()
        .find(|(children, _)| {
            children
                .iter()
                .any(|&child| primary_players.contains(child))
        })
        .expect("primary player exists in a level");

    for (level_children, level_pos) in levels
        .iter()
        .filter(|(_, level_pos)| level_pos.0.is_neighbor(primary_level_pos.0))
    {
        let (_, arrows_tile_storage) = level_children
            .iter()
            .filter_map(|&child| layers.get(child).ok())
            .find(|(metadata, _)| metadata.identifier == "ArrowsTo")
            .expect("ArrowsFrom layer entity exists");
        let mut set_arrow_visible = |grid_coords: GridCoords| {
            let tile_pos = grid_coords_to_tile_pos(grid_coords).expect("edge coords are in bounds");
            let arrow_tile = arrows_tile_storage
                .get(&tile_pos)
                .expect("arrow tile exists at edge coords");
            let mut tile_visible = tiles
                .get_mut(arrow_tile)
                .expect("arrow tile matches tile query");
            tile_visible.0 = true;
        };
        // level is below primary level, so set the top edge visible
        if level_pos.0.row == primary_level_pos.0.row + 1
            && level_pos.0.col == primary_level_pos.0.col
        {
            current_level
                .0
                .top_boundary_coords()
                .for_each(&mut set_arrow_visible);
        }
        // level is above primary level, so set the bottom edge visible
        if level_pos.0.row == primary_level_pos.0.row - 1
            && level_pos.0.col == primary_level_pos.0.col
        {
            current_level
                .0
                .bottom_boundary_coords()
                .for_each(&mut set_arrow_visible);
        }
        // level is to the right of primary level, so set the left edge visible
        if level_pos.0.row == primary_level_pos.0.row
            && level_pos.0.col == primary_level_pos.0.col + 1
        {
            current_level
                .0
                .left_boundary_coords()
                .for_each(&mut set_arrow_visible);
        }
        // level is to the left of primary level, so set the right edge visible
        if level_pos.0.row == primary_level_pos.0.row
            && level_pos.0.col == primary_level_pos.0.col - 1
        {
            current_level
                .0
                .right_boundary_coords()
                .for_each(&mut set_arrow_visible);
        }
    }
}
