use crate::GameState;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.add_loading_state(
            LoadingState::new(GameState::Loading).continue_to_state(GameState::MainMenu),
        )
        .add_collection_to_loading_state::<_, GameAssets>(GameState::Loading);
    }
}

#[derive(Resource, AssetCollection)]
pub struct GameAssets {
    #[asset(path = "levels/beside_yourself.ldtk")]
    pub levels: Handle<bevy_ecs_ldtk::LdtkAsset>,
    #[asset(path = "fonts/Kenney Pixel.ttf")]
    pub main_font: Handle<Font>,
    #[asset(path = "px.png")]
    pub pixel: Handle<Image>,
}
