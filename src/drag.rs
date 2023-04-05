use bevy::{prelude::*, ui::RelativeCursorPosition};

use crate::{
    level::{ActiveLevel, MetaGridPos},
    GameState,
};

pub struct DragPlugin;

impl Plugin for DragPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spawn_ui_root.in_schedule(OnEnter(GameState::InGame)))
            .add_systems(
                (
                    spawn_drag_areas.run_if(resource_changed::<ActiveLevel>()),
                    update_cursor_icon,
                )
                    .in_set(OnUpdate(GameState::InGame)),
            );
    }
}

// ====================
// ==== COMPONENTS ====
// ====================

#[derive(Component)]
struct UiRoot;

#[derive(Component)]
struct Container;

#[derive(Component)]
struct DragArea;

#[derive(Component)]
struct DragAreaPosition(MetaGridPos);

// =================
// ==== SYSTEMS ====
// =================

fn spawn_ui_root(mut commands: Commands) {
    commands.spawn(UiRoot).insert(NodeBundle {
        style: Style {
            size: Size::all(Val::Percent(100.)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        ..default()
    });
}

fn spawn_drag_areas(
    mut commands: Commands,
    active_level: Res<ActiveLevel>,
    ui_root_query: Query<Entity, With<UiRoot>>,
) {
    let ui_root = ui_root_query.single();
    commands.entity(ui_root).despawn_descendants();
    let container = commands
        .spawn(Container)
        .insert(NodeBundle {
            style: Style {
                size: Size::new(
                    Val::Px(active_level.width_px() as f32),
                    Val::Px(active_level.height_px() as f32),
                ),
                ..default()
            },
            ..default()
        })
        .id();
    commands.entity(ui_root).add_child(container);
    for col in 0..active_level.grid_width {
        for row in 0..active_level.grid_height {
            let drag_area = commands
                .spawn((DragArea, DragAreaPosition(MetaGridPos::new(row, col))))
                .insert(NodeBundle {
                    style: Style {
                        size: Size::new(
                            Val::Px((active_level.item_width_px - 2 * crate::GRID_SIZE) as f32),
                            Val::Px((active_level.item_height_px - 2 * crate::GRID_SIZE) as f32),
                        ),
                        margin: UiRect::all(Val::Px(crate::GRID_SIZE as f32)),
                        position_type: PositionType::Absolute,
                        position: UiRect {
                            top: Val::Px((row * active_level.item_height_px) as f32),
                            left: Val::Px((col * active_level.item_width_px) as f32),
                            ..default()
                        },
                        ..default()
                    },
                    ..default()
                })
                .insert(RelativeCursorPosition::default())
                .id();
            commands.entity(container).add_child(drag_area);
        }
    }
}

fn update_cursor_icon(
    mut windows: Query<&mut Window>,
    drag_areas: Query<&RelativeCursorPosition, With<DragArea>>,
) {
    let mut window = windows.single_mut();
    if drag_areas.iter().any(|drag_area| drag_area.mouse_over()) {
        window.cursor.icon = CursorIcon::Grab;
    } else {
        window.cursor.icon = CursorIcon::Default;
    }
}
