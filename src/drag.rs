use bevy::{
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::RenderLayers,
    },
    ui::RelativeCursorPosition,
};

use crate::{
    level::{CurrentMetaLevel, LevelPosition, MetaGridCoords},
    GameState, MainCamera, DRAG_RENDER_LAYER, MAIN_RENDER_LAYER,
};

pub struct DragPlugin;

impl Plugin for DragPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SwapLevelsEvent>()
            .add_systems(
                (spawn_ui_root, setup_image_render_target).in_schedule(OnEnter(GameState::InGame)),
            )
            .add_systems(
                (
                    swap_levels,
                    update_cursor_icon,
                    drag_icon,
                    spawn_drag_areas.run_if(resource_exists_and_changed::<CurrentMetaLevel>()),
                    begin_drag.run_if(not(resource_exists::<Dragging>())),
                    end_drag.run_if(resource_exists::<Dragging>()),
                )
                    .distributive_run_if(resource_exists::<CurrentMetaLevel>())
                    .in_set(OnUpdate(GameState::InGame)),
            );
    }
}

// ===================
// ==== RESOURCES ====
// ===================

#[derive(Resource)]
struct Dragging {
    from_pos: MetaGridCoords,
}

// ================
// ==== EVENTS ====
// ================

struct SwapLevelsEvent {
    from_pos: MetaGridCoords,
    to_pos: MetaGridCoords,
}

// ====================
// ==== COMPONENTS ====
// ====================

#[derive(Component)]
struct UiRenderCamera;

#[derive(Component)]
struct DragSprite;

#[derive(Component)]
struct UiRoot;

#[derive(Component)]
struct Container;

#[derive(Component)]
struct DragArea;

#[derive(Component)]
struct DragAreaPosition(MetaGridCoords);

// =================
// ==== SYSTEMS ====
// =================

fn swap_levels(
    current_level: Res<CurrentMetaLevel>,
    mut swap_events: EventReader<SwapLevelsEvent>,
    mut ldtk_levels: Query<(&mut LevelPosition, &mut Transform)>,
) {
    for event in swap_events.iter() {
        for (mut level_pos, mut transform) in &mut ldtk_levels {
            if level_pos.0 == event.from_pos {
                *level_pos = LevelPosition(event.to_pos);
                transform.translation = current_level.0.get_translation(event.to_pos).extend(0.);
            } else if level_pos.0 == event.to_pos {
                *level_pos = LevelPosition(event.from_pos);
                transform.translation = current_level.0.get_translation(event.from_pos).extend(0.);
            }
        }
    }
}

// adapted from https://github.com/bevyengine/bevy/blob/v0.10.1/examples/3d/render_to_texture.rs
fn setup_image_render_target(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let size = Extent3d {
        width: crate::WIDTH as u32,
        height: crate::HEIGHT as u32,
        ..default()
    };
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    image.resize(size);
    let image_handle = images.add(image);
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                target: RenderTarget::Image(image_handle.clone()),
                ..default()
            },
            ..default()
        },
        RenderLayers::layer(MAIN_RENDER_LAYER),
        UiRenderCamera,
    ));
    commands
        .spawn((DragSprite, RenderLayers::layer(DRAG_RENDER_LAYER)))
        .insert(SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(1., 1., 1., 0.5),
                ..default()
            },
            texture: image_handle.clone(),
            visibility: Visibility::Hidden,
            ..default()
        })
        .insert(Transform::from_translation(Vec3::new(0., 0., 10.)));
}

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
    current_level: Res<CurrentMetaLevel>,
    ui_root_query: Query<Entity, With<UiRoot>>,
) {
    let ui_root = ui_root_query.single();

    // remove any UI elements from the previous level
    commands.entity(ui_root).despawn_descendants();

    let container = commands
        .spawn(Container)
        .insert(NodeBundle {
            style: Style {
                size: Size::new(
                    Val::Px(current_level.0.total_width_px() as f32),
                    Val::Px(current_level.0.total_height_px() as f32),
                ),
                ..default()
            },
            ..default()
        })
        .id();
    commands.entity(ui_root).add_child(container);

    for col in 0..current_level.0.meta_grid_width {
        for row in 0..current_level.0.meta_grid_height {
            let drag_area = commands
                .spawn((DragArea, DragAreaPosition(MetaGridCoords::new(row, col))))
                .insert(NodeBundle {
                    style: Style {
                        size: Size::new(
                            Val::Px(current_level.0.unpadded_item_width_px() as f32),
                            Val::Px(current_level.0.unpadded_item_height_px() as f32),
                        ),
                        margin: UiRect::all(Val::Px(crate::GRID_SIZE as f32)),
                        position_type: PositionType::Absolute,
                        position: UiRect {
                            top: Val::Px((row * current_level.0.level_height_px()) as f32),
                            left: Val::Px((col * current_level.0.level_width_px()) as f32),
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
    dragging: Option<Res<Dragging>>,
    mut windows: Query<&mut Window>,
    drag_areas: Query<&RelativeCursorPosition, With<DragArea>>,
) {
    let mut window = windows.single_mut();
    window.cursor.icon = if dragging.is_some() {
        CursorIcon::Grabbing
    } else if drag_areas
        .iter()
        .any(|rel_cursor_pos| rel_cursor_pos.mouse_over())
    {
        CursorIcon::Grab
    } else {
        CursorIcon::Default
    };
}

fn begin_drag(
    mut commands: Commands,
    current_level: Res<CurrentMetaLevel>,
    input: Res<Input<MouseButton>>,
    drag_areas: Query<(&RelativeCursorPosition, &DragAreaPosition)>,
    mut drag_sprite: Query<&mut Sprite, With<DragSprite>>,
) {
    if input.just_pressed(MouseButton::Left) {
        for (rel_cursor_pos, drag_area_pos) in drag_areas.iter() {
            if rel_cursor_pos.mouse_over() {
                commands.insert_resource(Dragging {
                    from_pos: drag_area_pos.0,
                });
                let mut sprite = drag_sprite.single_mut();
                let foo = Some(Rect::from_center_size(
                    current_level
                        .0
                        .get_center_translation_for_texture(drag_area_pos.0),
                    Vec2::new(
                        current_level.0.unpadded_item_width_px() as f32,
                        current_level.0.unpadded_item_height_px() as f32,
                    ),
                ));
                sprite.rect = foo;
            }
        }
    }
}

fn drag_icon(
    dragging: Option<Res<Dragging>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut drag_sprite: Query<(&mut Transform, &mut Visibility), With<DragSprite>>,
) {
    let (mut sprite_transform, mut sprite_visibility) = drag_sprite.single_mut();
    if dragging.is_some() {
        // from https://bevy-cheatbook.github.io/cookbook/cursor2world.html
        let window = windows.single();
        let (camera, camera_transform) = cameras.single();
        if let Some(mouse_world_pos) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
            .map(|ray| ray.origin.truncate())
        {
            // round the mouse coords to the nearest pixel to ensure pixel art is crisp
            sprite_transform.translation =
                Vec3::new(mouse_world_pos.x.round(), mouse_world_pos.y.round(), 10.);
        }
        *sprite_visibility = Visibility::Visible;
    } else {
        *sprite_visibility = Visibility::Hidden;
    }
}

fn end_drag(
    mut commands: Commands,
    input: Res<Input<MouseButton>>,
    dragging: Res<Dragging>,
    drag_areas: Query<(&RelativeCursorPosition, &DragAreaPosition)>,
    mut drag_sprite: Query<&mut Visibility, With<DragSprite>>,
    mut swap_events: EventWriter<SwapLevelsEvent>,
) {
    if input.just_released(MouseButton::Left) {
        let mut sprite_visibility = drag_sprite.single_mut();
        *sprite_visibility = Visibility::Hidden;
        for (rel_cursor_pos, drag_area_pos) in drag_areas.iter() {
            if rel_cursor_pos.mouse_over() {
                swap_events.send(SwapLevelsEvent {
                    from_pos: dragging.from_pos,
                    to_pos: drag_area_pos.0,
                })
            }
        }
        commands.remove_resource::<Dragging>();
    }
}
