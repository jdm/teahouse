use bevy::prelude::*;
use bevy::sprite::collide_aabb::collide;
use bevy::time::FixedTimestep;
use std::default::Default;

// Defines the amount of time that should elapse between each physics step.
const TIME_STEP: f32 = 1.0 / 60.0;

fn main() {
    let map = read_map(MAP);

    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                .with_system(check_for_collisions)
                .with_system(halt_collisions.after(check_for_collisions))
                .with_system(move_movable.after(halt_collisions))

        )
        .insert_resource(map)
        .add_system(select_pathfinding_targets)
        .add_system(pathfind)
        .add_system(keyboard_input)
        .add_system(bevy::window::close_on_esc)
        .add_event::<CollisionEvent>()
        .add_event::<PathfindEvent>()
        .run();
}

struct CollisionEvent {
    moving: Entity,
    _fixed: Entity,
}

#[derive(Component)]
struct Movable {
    speed: Vec2,
    size: Vec2,
}

#[derive(Component)]
struct Player;

#[derive(PartialEq)]
enum CustomerState {
    LookingForChair,
    SittingInChair,
}

#[derive(Component)]
struct Customer {
    goal: Option<MapPos>,
    path: Option<Vec<MapPos>>,
    state: CustomerState,
}

impl Default for Customer {
    fn default() -> Self {
        Self {
            goal: None,
            path: None,
            state: CustomerState::LookingForChair,
        }
    }
}

#[derive(Component)]
struct Prop;

#[derive(Component)]
struct Chair {
    pos: MapPos,
    occupied: bool,
}

struct PathfindEvent {
    customer: Entity,
    destination: MapPos,
}

fn select_pathfinding_targets(
    mut q: Query<(Entity, &mut Customer, &mut Movable, &mut Transform)>,
    mut chairs: Query<&mut Chair>,
    mut pathfind_events: EventWriter<PathfindEvent>,
    map: Res<Map>
) {
    for (entity, mut customer, mut movable, mut transform) in &mut q {
        if customer.goal.is_none() && customer.state == CustomerState::LookingForChair {
            for chair in &chairs {
                if !chair.occupied {
                    println!("giving customer a goal: {:?}", chair.pos);
                    pathfind_events.send(PathfindEvent {
                        customer: entity,
                        destination: chair.pos,
                    });
                    break;
                }
            }
        } else if let Some(point) = customer.path.as_ref().and_then(|path| path.first().cloned()) {
            let current_point = transform_to_map_pos(&transform, &map);
            let ideal_point = map_to_screen(&current_point, &MapSize { width: 1, height: 1 }, &map);
            println!("screen point: {:?}, current point: {:?}, ideal point: {:?}, next goal: {:?}", transform.translation, current_point, ideal_point, point);
            if current_point == point {
                println!("reached target point, resetting");
                customer.path.as_mut().unwrap().remove(0);
                transform.translation = Vec2::new(ideal_point.x, ideal_point.y).extend(0.);
                movable.speed = Vec2::ZERO;
            } else {
                if point.x < current_point.x {
                    movable.speed.x = -CUSTOMER_SPEED;
                } else if point.x > current_point.x {
                    movable.speed.x = CUSTOMER_SPEED;
                }
                if point.y < current_point.y {
                    movable.speed.y = CUSTOMER_SPEED;
                } else if point.y > current_point.y {
                    movable.speed.y = -CUSTOMER_SPEED;
                }
            }
        } else if let Some(goal) = customer.goal.clone() {
            let current_point = transform_to_map_pos(&transform, &map);
            println!("current point: {:?}, terminal goal: {:?}", current_point, goal);
            assert_eq!(current_point, goal);
            customer.path = None;
            customer.goal = None;
            for mut chair in &mut chairs {
                if chair.pos == goal {
                    chair.occupied = true;
                    customer.state = CustomerState::SittingInChair;
                    break;
                }
            }
        }
    }
}

fn pathfind(
    mut pathfind_events: EventReader<PathfindEvent>,
    mut q: Query<(Entity, &mut Customer, &Transform)>,
    map: Res<Map>,
) {
    if pathfind_events.is_empty() {
        return;
    }
    
    let mut tiles = vec![vec![1; map.width]; map.height];
    for prop in &map.props {
        for y in 0..prop.0.height {
            for x in 0..prop.0.width {
                tiles[y + prop.1.y][x + prop.1.x] = 0;
            }
        }
    }
    let grid = basic_pathfinding::grid::Grid {
        tiles,
        walkable_tiles: vec![1],
        grid_type: basic_pathfinding::grid::GridType::Cardinal,
        ..default()
    };
    // TODO: add customer and player positions

    for ev in pathfind_events.iter() {
        for (entity, mut customer, transform) in &mut q {
            if entity != ev.customer {
                continue;
            }

            let start = transform_to_map_pos(transform, &map);
            let start_grid = basic_pathfinding::coord::Coord::new(start.x as i32, start.y as i32);
            let end = basic_pathfinding::coord::Coord::new(ev.destination.x as i32, ev.destination.y as i32);
            let path = basic_pathfinding::pathfinding::find_path(&grid, start_grid, end, Default::default());
            println!("path from {:?} to {:?}: {:?}", start, ev.destination, path);
            if path.is_none() {
                println!("no path to goal!");
                break;
            }
            customer.goal = Some(ev.destination);
            customer.path = Some(path
                .unwrap()
                .into_iter()
                .map(|point| MapPos { x: point.x as usize, y: point.y as usize })
                .collect());
            break;
        }
    }
    pathfind_events.clear();
}

fn check_for_collisions(
    q: Query<(Entity, &Movable, &Transform)>,
    timer: Res<Time>,
    mut collision_events: EventWriter<CollisionEvent>,
) {
    for (moving, movable, transform) in &q {
        for (fixed, movable2, transform2) in &q {
            if moving == fixed {
                continue;
            }

            if movable.speed == Vec2::ZERO {
                continue;
            }

            let delta = movable.speed.extend(0.);
            let adjusted = transform.translation + delta * timer.delta_seconds();

            let collision = collide(adjusted, movable.size, transform2.translation, movable2.size);
            if collision.is_some() {
                println!("collision for {:?} and {:?}", moving, fixed);
                collision_events.send(CollisionEvent {
                    moving: moving,
                    _fixed: fixed,
                });
            }
        }
    }
}

fn halt_collisions(
    mut collision_events: EventReader<CollisionEvent>,
    mut q: Query<(Entity, &mut Movable)>,
) {
    for ev in collision_events.iter() {
        for (entity, mut movable) in &mut q {
            if ev.moving == entity {
                movable.speed = Vec2::ZERO;
            }
        }
    }
    collision_events.clear();
}

fn move_movable(mut q: Query<(&Movable, &mut Transform)>, timer: Res<Time>) {
    for (movable, mut transform) in &mut q {
        let delta = Vec3::new(movable.speed.x, movable.speed.y, 0.0);
        transform.translation += delta * timer.delta_seconds();
    }
}

const CUSTOMER_SPEED: f32 = 25.0;
const SPEED: f32 = 150.0;
const TILE_SIZE: f32 = 25.0;

fn keyboard_input(
    keys: Res<Input<KeyCode>>,
    mut q: Query<(&Player, &mut Movable)>,
) {
    let (_player, mut movable) = q.single_mut();

    if keys.just_pressed(KeyCode::Up) {
        movable.speed.y = SPEED;
    } else if keys.just_pressed(KeyCode::Down) {
        movable.speed.y = -SPEED;
    }
    if keys.any_just_released([KeyCode::Up, KeyCode::Down]) {
        movable.speed.y = 0.0;
    }

    if keys.just_pressed(KeyCode::Left) {
        movable.speed.x = -SPEED;
    } else if keys.just_pressed(KeyCode::Right) {
        movable.speed.x = SPEED;
    }
    if keys.any_just_released([KeyCode::Left, KeyCode::Right]) {
        movable.speed.x = 0.0;
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum EntityType {
    Customer,
    Player,
    Prop,
    Chair,
}

fn spawn_sprite(entity: EntityType, rect: ScreenRect, map_pos: MapPos, commands: &mut Commands) {
    let pos = Vec2::new(rect.x, rect.y);
    let size = Vec2::new(rect.w, rect.h);
    let color = match entity {
        EntityType::Player => Color::rgb(0.25, 0.25, 0.75),
        EntityType::Customer => Color::rgb(0.0, 0.25, 0.0),
        EntityType::Prop => Color::rgb(0.25, 0.15, 0.0),
        EntityType::Chair => Color::rgb(0.15, 0.05, 0.0),
    };
    let sprite = SpriteBundle {
        sprite: Sprite {
            color,
            custom_size: Some(size),
            ..default()
        },
        transform: Transform::from_translation(pos.extend(0.)),
        ..default()
    };
    let movable = Movable { speed: Vec2::ZERO, size: size };
    match entity {
        EntityType::Player => {
            commands.spawn((Player, movable, sprite))
                .with_children(|parent| {
                    parent.spawn(Camera2dBundle::default());
                });
        }
        EntityType::Customer => {
            commands.spawn((Customer::default(), movable, sprite));
        }
        EntityType::Chair => {
            commands.spawn((Chair {
                pos: map_pos,
                occupied: false,
            }, sprite));
        }
        EntityType::Prop => {
            commands.spawn((Prop, movable, sprite));
        }
    };
}

static MAP: &[&str] = &[
    ".................................................",
    ".xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.",
    ".x.............................................x.",
    ".x.............................................x.",
    ".x..........c..................................x.",
    ".x........cxxx.................................x.",
    ".x.........xxxc................................x.",
    ".x..........c..................................x.",
    ".x....xxx......................................x.",
    ".x.....C.......................................x.",
    ".x.............................................x.",
    ".x.....................P.......................x.",
    ".x.............................................x.",
    ".xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.",
    ".................................................",
];

#[derive(Default, PartialEq, Debug, Copy, Clone)]
struct MapPos {
    x: usize,
    y: usize,
}

#[derive(PartialEq, Debug, Copy, Clone)]
struct ScreenRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Default, PartialEq, Debug)]
struct MapSize {
    width: usize,
    height: usize,
}

#[derive(Default, PartialEq, Debug, Resource)]
struct Map {
    entities: Vec<(EntityType, MapPos)>,
    props: Vec<(MapSize, MapPos)>,
    chairs: Vec<MapPos>,
    width: usize,
    height: usize,
}

fn read_map(data: &[&str]) -> Map {
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
                map.entities.push((EntityType::Customer, MapPos { x, y }));
            } else if ch == 'c' {
                map.chairs.push(MapPos { x, y });
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

// FIXME: only valid for 1x1 entities
fn screen_to_map_pos(x: f32, y: f32, map: &Map) -> MapPos {
    let x = (x - TILE_SIZE / 2.) / TILE_SIZE + map.width as f32 / 2.;
    let y = -((y + TILE_SIZE / 2.) / TILE_SIZE - map.height as f32 / 2.);
    assert!(x >= 0.);
    assert!(y >= 0.);
    MapPos { x: x as usize, y: y as usize }
}

fn transform_to_map_pos(transform: &Transform, map: &Map) -> MapPos {
    let translation = transform.translation;
    screen_to_map_pos(translation.x, translation.y, map)
}

fn map_to_screen(pos: &MapPos, size: &MapSize, map: &Map) -> ScreenRect {
    let middle = ((map.width / 2) as f32, (map.height / 2) as f32);
    let screen_origin = Vec2::new(
        pos.x as f32 - middle.0,
        middle.1 as f32 - pos.y as f32,
    ) * TILE_SIZE;
    let screen_size = (
        size.width as f32 * TILE_SIZE,
        size.height as f32 * TILE_SIZE,
    );
    // Adjust from a center origin to a top-left origin.
    let origin_offset = Vec2::new(screen_size.0 / 2., -(screen_size.1 / 2.));
    let adjusted = screen_origin + origin_offset;
    ScreenRect {
        x: adjusted.x,
        y: adjusted.y,
        w: screen_size.0,
        h: screen_size.1,
    }
}

fn setup(
    mut commands: Commands,
    map: Res<Map>,
) {
    for pos in &map.chairs {
        let rect = map_to_screen(pos, &MapSize { width: 1, height: 1 }, &map);
        spawn_sprite(
            EntityType::Chair,
            rect,
            *pos,
            &mut commands,
        )
        
    }

    for (size, pos) in &map.props {
        let rect = map_to_screen(pos, size, &map);
        spawn_sprite(
            EntityType::Prop,
            rect,
            *pos,
            &mut commands,
        )
    }

    for (entity_type, pos) in &map.entities {
        let size = MapSize { width: 1, height: 1 };
        let rect = map_to_screen(pos, &size, &map);
        spawn_sprite(
            *entity_type,
            rect,
            *pos,
            &mut commands,
        );
    }
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
            (EntityType::Customer, MapPos { x: 5, y: 0 }),
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

#[test]
fn coord_conversion() {
    let map = Map { width: 18, height: 8, ..default() };
    assert_eq!(map_to_screen(
        &MapPos { x: 0, y: 0 },
        &MapSize { width: 4, height: 1 },
        &map
    ), ScreenRect { x: -175., y: 87.5, w: 100., h: 25. });
}

#[test]
fn screen_to_map_subtile_x() {
    let map = Map { width: 8, height: 8, ..default() };
    let mut screen_coords = map_to_screen(
        &MapPos { x: 4, y: 4 },
        &MapSize { width: 1, height: 1 },
        &map
    );
    for _ in 0..TILE_SIZE as usize {
        assert_eq!(
            screen_to_map_pos(screen_coords.x, screen_coords.y, &map),
            MapPos { x: 4, y: 4 },
        );
        screen_coords.x += 1.;
    }

    assert_eq!(
        screen_to_map_pos(screen_coords.x, screen_coords.y, &map),
        MapPos { x: 5, y: 4 },
    );
}

#[test]
fn screen_to_map_subtile_y() {
    let map = Map { width: 8, height: 8, ..default() };
    let mut screen_coords = map_to_screen(
        &MapPos { x: 4, y: 4 },
        &MapSize { width: 1, height: 1 },
        &map
    );
    for _ in 0..TILE_SIZE as usize {
        assert_eq!(
            screen_to_map_pos(screen_coords.x, screen_coords.y, &map),
            MapPos { x: 4, y: 4 },
        );
        screen_coords.y -= 1.;
    }

    assert_eq!(
        screen_to_map_pos(screen_coords.x, screen_coords.y, &map),
        MapPos { x: 4, y: 5 },
    );
}
