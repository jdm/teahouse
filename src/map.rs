use bevy::prelude::*;
use crate::entity::EntityType;
use crate::geom::{MapPos, MapSize};
use tiled::Loader;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        let map = read_map();
        app.insert_resource(map);
    }
}

#[derive(Default, PartialEq, Debug, Resource)]
pub struct Map {
    pub _entities: Vec<(EntityType, MapPos)>,
    pub _props: Vec<(MapSize, MapPos)>,
    pub _cupboards: Vec<MapPos>,
    pub _cat_beds: Vec<MapPos>,
    pub width: usize,
    pub height: usize,
}

fn read_map() -> Map {
    let mut loader = Loader::new();
    let map = loader.load_tmx_map("assets/teahouse.tmx").unwrap();

    let map = Map {
        width: map.width as usize,
        height: map.height as usize,
        ..default()
    };

    map
}
