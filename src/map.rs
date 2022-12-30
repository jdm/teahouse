use bevy::prelude::*;
use crate::entity::*;
use crate::geom::*;

pub static MAP: &[&str] = &[
    "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxB.xxxxxxxxxxx",
    "xb....................................xsssxxxxx",
    "x.k.............P............................tx",
    "x.....C....c......................xx.........tx",
    "x........cxxx...........c...........xxxxx....tx",
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

pub fn read_map(data: &[&str]) -> Map {
    let mut map = Map {
        height: data.len(),
        width: data[0].len(),
        ..default()
    };

    for (y, line) in data.iter().enumerate() {
        let mut chars = line.chars().enumerate().peekable();
        while let Some((x, ch)) = chars.next() {
            if ch == 'P' {
                map.entities.push((EntityType::Player, MapPos { x, y }));
            } else if ch == 'C' {
                map.entities.push((EntityType::Customer(vec![
                    "first".to_string(), "second".to_string(), "third".to_string(),
                ]), MapPos { x, y }));
            } else if ch == 'c' {
                let pos = MapPos { x, y };
                map.entities.push((EntityType::Chair(pos), pos));
            } else if ch == 'B' {
                map.cupboards.push(MapPos { x, y });
            } else if ch == 'D' {
                map.entities.push((EntityType::Door, MapPos { x, y }));
            } else if ch == 's' {
                map.entities.push((EntityType::Stove, MapPos { x, y }));
            } else if ch == 't' {
                map.entities.push((
                    EntityType::TeaStash(Ingredient::generate_random(), rand::random()),
                    MapPos { x, y },
                ));
            } else if ch == 'b' {
                map.cat_beds.push(MapPos { x, y });
            } else if ch == 'k' {
                map.entities.push((EntityType::Cat, MapPos { x, y }));
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
    static TEST: &[&str] = &["..P..C.."];
    let map = read_map(TEST);
    let expected = Map {
        width: 8,
        height: 1,
        entities: vec![
            (EntityType::Player, MapPos { x: 2, y: 0 }),
            (EntityType::Customer(vec![
                "first".to_string(), "second".to_string(), "third".to_string()
            ]), MapPos { x: 5, y: 0 }),
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
