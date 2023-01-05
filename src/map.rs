use bevy::prelude::*;
use bevy::asset::{AssetLoader, AssetPath, LoadedAsset, LoadState};
use bevy::reflect::TypeUuid;
use crate::GameState;
use std::io::BufReader;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_asset::<TiledMap>()
            .add_asset_loader(TiledLoader)
            .add_system_set(
                SystemSet::on_update(crate::GameState::Loading)
                    .with_system(transition_from_loading)
            )
            .add_system_set(
                SystemSet::on_enter(crate::GameState::Processing)
                    .with_system(crate::entity::setup2)
            )
            .add_system_set(
                SystemSet::on_update(crate::GameState::Processing)
                    .with_system(crate::entity::start_game)
            )
            ;
    }
}

fn transition_from_loading(
    map: Res<Map>,
    asset_server: Res<AssetServer>,
    mut game_state: ResMut<State<crate::GameState>>,
) {
    if asset_server.get_load_state(&map.handle) == LoadState::Loaded {
        game_state.set(GameState::Processing).unwrap();
    }
}

#[derive(Default, PartialEq, Debug, Resource)]
pub struct Map {
    pub width: usize,
    pub height: usize,
    pub handle: Handle<TiledMap>,
}

#[derive(TypeUuid)]
#[uuid = "e51081d0-6168-4881-a1c6-4249b2000d7f"]
pub struct TiledMap {
    pub map: tiled::Map,
    pub tilesets: Vec<Handle<Image>>,
}

pub struct TiledLoader;

impl AssetLoader for TiledLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::asset::BoxedFuture<'a, anyhow::Result<(), anyhow::Error>> {
        Box::pin(async move {
            let root_dir = load_context.path().parent().unwrap();
            let mut loader = tiled::Loader::new();
            let map = loader.load_tmx_map_from(BufReader::new(bytes), load_context.path())?;

            let mut dependencies = Vec::new();
            let mut handles = Vec::new();

            for tileset in map.tilesets().iter() {
                let img = tileset.image.as_ref().unwrap();
                let tile_path = root_dir.join(&img.source);
                let asset_path = AssetPath::new(tile_path, None);
                let texture: Handle<Image> = load_context.get_handle(asset_path.clone());
                handles.push(texture);
                dependencies.push(asset_path);
            }

            let loaded_asset = LoadedAsset::new(TiledMap {
                map,
                tilesets: handles,
            });
            load_context.set_default_asset(loaded_asset.with_dependencies(dependencies));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["tmx"];
        EXTENSIONS
    }
}
