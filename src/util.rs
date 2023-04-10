use bevy::{
    ecs::{
        archetype::Archetypes,
        component::{ComponentId, Components},
    },
    prelude::*,
};
use bevy_ecs_tilemap::tiles::TilePos;

pub struct UtilPlugin;

impl Plugin for UtilPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DebugEntityEvent>()
            .add_systems((entity_component_debugger,));
    }
}

// ================
// ==== EVENTS ====
// ================

pub struct DebugEntityEvent {
    pub entity: Entity,
}

// =================
// ==== SYSTEMS ====
// =================

fn entity_component_debugger(
    mut events: EventReader<DebugEntityEvent>,
    archetypes: &Archetypes,
    components: &Components,
) {
    for event in events.iter() {
        let component_names = get_components_for_entity(&event.entity, archetypes)
            .unwrap()
            .filter_map(|component| components.get_info(component))
            .map(|info| info.name())
            .collect::<Vec<_>>();
        info!("Components: {component_names:?}");
    }
}

pub fn grid_coords_to_tile_pos(
    grid_coords: bevy_ecs_ldtk::GridCoords,
) -> Option<bevy_ecs_tilemap::tiles::TilePos> {
    let x = u32::try_from(grid_coords.x).ok()?;
    let y = u32::try_from(grid_coords.y).ok()?;
    Some(TilePos::new(x, y))
}

// from https://github.com/bevyengine/bevy/discussions/3332
fn get_components_for_entity<'a>(
    entity: &Entity,
    archetypes: &'a Archetypes,
) -> Option<impl Iterator<Item = ComponentId> + 'a> {
    for archetype in archetypes.iter() {
        if archetype.entities().iter().any(|e| e.entity() == *entity) {
            return Some(archetype.components());
        }
    }
    None
}
