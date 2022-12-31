use bevy::prelude::*;
use crate::entity::EntityType;
use crate::geom::{MapPos, MapSize};
use crate::tea::Ingredient;
use rand::Rng;
use std::collections::HashMap;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        let map = read_map(MAP);
        app.insert_resource(map);
    }
}

static MAP: &[&str] = &[
    "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxB.xxxxxxxxxxx",
    "xb....................................xxxxxxxxx",
    "x.k.............P............................tx",
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
}

fn read_map(data: &[&str]) -> Map {
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
        ('c', EntityType::Chair),
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
