use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use crate::animation::{AnimData, TextureResources};
use crate::entity::{EntityType, TileDirection};
use crate::geom::{MapPos, MapSize, TILE_SIZE};
use crate::tea::Ingredient;
use rand::Rng;
use std::collections::HashMap;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        //let map = read_map(MAP);
        app
            .add_startup_system(setup_map);
            //.insert_resource(map);
    }
}

static MAP: &[&str] = &[
    "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxB.xxxxxxxxxxx",
    "xb....................................xxxxxxxxx",
    "x.k.............P...T........................tx",
    "x..........c......................xx.........tx",
    "x........cxxx...........c...........xxxKx....tx",
    "x.........xxxc.........xx..............xxxx...x",
    "D..........c..........xxxxc.......c...........x",
    "x....xxx.............cxxx........xxx..........x",
    "x....xxx......................cxxxxxxxc.......x",
    "x................................xxx..........x",
    "x.................................c...........x",
    "x.............................................x",
    "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
];

#[derive(Default, PartialEq, Debug, Resource)]
pub struct Map {
    pub entities: Vec<(EntityType, MapPos)>,
    pub props: Vec<(MapSize, MapPos)>,
    pub cupboards: Vec<MapPos>,
    pub cat_beds: Vec<MapPos>,
    pub width: usize,
    pub height: usize,
    pub collision_map: Vec<Vec<i32>>,
}

pub fn setup_map(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let texture_handle = asset_server.load("cat.png");
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, Vec2::new(TILE_SIZE, TILE_SIZE), 4, 5, None, None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    let texture_handle2 = asset_server.load("interiors.png");
    let texture_atlas2 =
        TextureAtlas::from_grid(texture_handle2.clone(), Vec2::new(TILE_SIZE, TILE_SIZE), 48, 16, None, None);
    let texture_atlas_handle2 = texture_atlases.add(texture_atlas2);

    let texture_resources = TextureResources {
        atlas: texture_atlas_handle,
        interior_atlas: texture_atlas_handle2,
        frame_data: vec![
            AnimData { index: 0, frames: 4, delay: 0.1, },
            AnimData { index: 4, frames: 4, delay: 0.1, },
            AnimData { index: 8, frames: 4, delay: 0.1, },
            AnimData { index: 12, frames: 4, delay: 0.1, },
            AnimData { index: 16, frames: 1, delay: 0.1, },
            AnimData { index: 17, frames: 2, delay: 0.8, },
        ],
    };

    let tilemap_entity = commands.spawn_empty().id();
    let map_size = TilemapSize {
        x: MAP[0].len() as u32,
        y: MAP.len() as u32,
    };
    let mut tile_storage = TileStorage::empty(map_size);

    let mut collision_tiles = vec![vec![1; MAP[0].len()]; MAP.len()];

    for (y, line) in MAP.iter().enumerate() {
        for (x, ch) in line.chars().enumerate() {
            let index = if ch == 'x' {
                collision_tiles[y][x] = 0;
                (2, 2)
            } else {
                (4, 8)
            };
            let tile_pos = TilePos { x: x as u32, y: y as u32 };
            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(tilemap_entity),
                    texture_index: TileTextureIndex(index.1 * 48 + index.0),
                    ..Default::default()
                })
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    let tile_size = TilemapTileSize { x: TILE_SIZE, y: TILE_SIZE };
    let grid_size = tile_size.into();
    let map_type = TilemapType::default();

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture_handle2.clone()),
        tile_size,
        transform: get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0),
        ..Default::default()
     });

    let mut map = read_map(MAP, &mut commands);
    map.collision_map = collision_tiles;

    crate::entity::setup(&mut commands, &mut map, &texture_resources);

    commands.insert_resource(map);
    commands.insert_resource(texture_resources);
}

fn read_map(
    data: &[&str],
    _commands: &mut Commands,
) -> Map {
    let mut map = Map {
        height: data.len(),
        width: data[0].len(),
        ..default()
    };

    let simple_entities = HashMap::from([
        ('P', EntityType::Player),
        ('D', EntityType::Door),
        ('s', EntityType::Stove),
        ('K', EntityType::Kettle),
        ('k', EntityType::Cat),
        ('T', EntityType::TeaPot),
    ]);

    let mut rng = rand::thread_rng();

    for (y, line) in data.iter().enumerate() {
        let mut chars = line.chars().enumerate().peekable();
        while let Some((x, ch)) = chars.next() {
            if simple_entities.contains_key(&ch) {
                map.entities.push((simple_entities[&ch].clone(), MapPos { x, y }));
            } else if ch == 'B' {
                map.cupboards.push(MapPos { x, y });
            } else if ch == 't' {
                map.entities.push((
                    EntityType::TeaStash(Ingredient::generate_random(), rng.gen_range(1..10)),
                    MapPos { x, y },
                ));
            } else if ch == 'b' {
                map.cat_beds.push(MapPos { x, y });
            } else if ch == 'c' {
                let dir = if data[y].as_bytes()[x-1] == b'x' {
                    TileDirection::Left
                } else if data[y].as_bytes()[x+1] == b'x' {
                    TileDirection::Right
                } else if data[y-1].as_bytes()[x] == b'x' {
                    TileDirection::Up
                } else {
                    TileDirection::Down
                };
                map.entities.push((EntityType::Chair(dir), MapPos { x, y }));
            } else if ch == 'x' {
                let mut length = 1;
                while let Some((_, 'x')) = chars.peek() {
                    let _ = chars.next();
                    length += 1;
                }

                let mut found = false;
                for prop in &mut map.props {
                    // Starts in a different column.
                    if prop.1.x != x {
                        continue
                    }
                    // Widths don't match.
                    if prop.0.width != length {
                        continue;
                    }
                    // Is not vertically adjacent.
                    if prop.1.y + prop.0.height != y {
                        continue;
                    }

                    // This row is an extension of a prop in the previous row.
                    prop.0.height += 1;
                    found = true;
                    break;
                }
                if !found {
                    map.props.push((
                        MapSize {
                            width: length,
                            height: 1,
                        },
                        MapPos {
                            x,
                            y,
                        },
                    ));
                }
            }
        }
    }
    map
}

#[test]
fn map_read_entities() {
    static TEST: &[&str] = &["..P..c.."];
    let map = read_map(TEST);
    let expected = Map {
        width: 8,
        height: 1,
        entities: vec![
            (EntityType::Player, MapPos { x: 2, y: 0 }),
            (EntityType::Chair, MapPos { x: 5, y: 0 }),
        ],
        ..default()
    };
    assert_eq!(map, expected);
}

#[test]
fn map_read_props_single() {
    static TEST: &[&str] = &["xxx..xxx"];
    let map = read_map(TEST);
    let expected = Map {
        width: 8,
        height: 1,
        props: vec![
            (MapSize { width: 3, height: 1 }, MapPos { x: 0, y: 0 }),
            (MapSize { width: 3, height: 1 }, MapPos { x: 5, y: 0 }),
        ],
        ..default()
    };
    assert_eq!(map, expected);
}

#[test]
fn map_read_props_double() {
    static TEST: &[&str] = &[
        "xxx..xxx",
        "xxx..xx.",
    ];
    let map = read_map(TEST);
    let expected = Map {
        width: 8,
        height: 2,
        props: vec![
            (MapSize { width: 3, height: 2 }, MapPos { x: 0, y: 0 }),
            (MapSize { width: 3, height: 1 }, MapPos { x: 5, y: 0 }),
            (MapSize { width: 2, height: 1 }, MapPos { x: 5, y: 1 }),
        ],
        ..default()
    };
    assert_eq!(map, expected);
}

#[test]
fn map_read_props_multi() {
    static TEST: &[&str] = &[
        "xxxx",
        "x..x",
        "x.Px",
        "x..x",
        "xxxx",
    ];
    let map = read_map(TEST);
    let expected = Map {
        width: 4,
        height: 5,
        entities: vec![
            (EntityType::Player, MapPos { x: 2, y: 2 }),
        ],
        props: vec![
            (MapSize { width: 4, height: 1 }, MapPos { x: 0, y: 0 }),
            (MapSize { width: 1, height: 3 }, MapPos { x: 0, y: 1 }),
            (MapSize { width: 1, height: 3 }, MapPos { x: 3, y: 1 }),
            (MapSize { width: 4, height: 1 }, MapPos { x: 0, y: 4 }),
        ],
        ..default()
    };
    assert_eq!(map, expected);
}
