use bevy::prelude::*;
use bevy_embedded_assets::EmbeddedAssetPlugin;

pub const WIDTH: f32 = 720.;
pub const HEIGHT: f32 = 576.;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            DefaultPlugins
                .build()
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: (WIDTH, HEIGHT).into(),
                        canvas: Some("#bevy".to_owned()),
                        ..default()
                    }),
                    ..default()
                })
                .add_before::<AssetPlugin, _>(EmbeddedAssetPlugin),
        );
    }
}
