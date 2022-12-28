use bevy::prelude::*;
use bevy::sprite::collide_aabb::collide;
use bevy::time::FixedTimestep;

// Defines the amount of time that should elapse between each physics step.
const TIME_STEP: f32 = 1.0 / 60.0;

fn main() {
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
        .add_system(keyboard_input)
        .add_system(bevy::window::close_on_esc)
        .add_event::<CollisionEvent>()
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

#[derive(Component)]
struct Customer;

#[derive(Component)]
struct Prop;

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
}

fn spawn_sprite(entity: EntityType, rect: ScreenRect, commands: &mut Commands) {
    let pos = Vec2::new(rect.x, rect.y);
    let size = Vec2::new(rect.w, rect.h);
    let color = match entity {
        EntityType::Player => Color::rgb(0.25, 0.25, 0.75),
        EntityType::Customer => Color::rgb(0.0, 0.25, 0.0),
        EntityType::Prop => Color::rgb(0.25, 0.15, 0.0),
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
                /*.with_children(|parent| {
                    parent.spawn(Camera2dBundle::default());
                })*/;
        }
        EntityType::Customer => {
            commands.spawn((Customer, movable, sprite));
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
    ".x.............................................x.",
    ".x.............................................x.",
    ".x.........xx..................................x.",
    ".x.........xx..................................x.",
    ".x.............................................x.",
    ".x.............................................x.",
    ".x.....C.......................................x.",
    ".x.............................................x.",
    ".x.............................................x.",
    ".x..................xxxxxx.....................x.",
    ".x.............................................x.",
    ".x.....................P.......................x.",
    ".x.............................................x.",
    ".x.............................................x.",
    ".x.............................................x.",
    ".x.............................................x.",
    ".xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.",
    ".................................................",
];

static MAP2: &[&str] = &[
    "xxxxxxxxxxxxxxxxxx",
    "x.........xx.....x",
    "x.........xx.....x",
    "x................x",
    "x.....C..........x",
    "x.......xxxxxx...x",
    "x..........P.....x",
    "xxxxxxxxxxxxxxxxxx",
];

#[derive(Default, PartialEq, Debug)]
struct MapPos {
    x: usize,
    y: usize,
}

#[derive(PartialEq, Debug)]
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

#[derive(Default, PartialEq, Debug)]
struct Map {
    entities: Vec<(EntityType, MapPos)>,
    props: Vec<(MapSize, MapPos)>,
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
    mut _meshes: ResMut<Assets<Mesh>>,
    mut _materials: ResMut<Assets<ColorMaterial>>,
) {
    let map = read_map(MAP2);

    commands.spawn(Camera2dBundle::default());

    for (entity_type, pos) in &map.entities {
        let size = MapSize { width: 1, height: 1 };
        let rect = map_to_screen(pos, &size, &map);
        spawn_sprite(
            *entity_type,
            rect,
            &mut commands,
        );
    }

    for (size, pos) in &map.props {
        let rect = map_to_screen(pos, size, &map);
        spawn_sprite(
            EntityType::Prop,
            rect,
            &mut commands,
        )
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
        props: vec![],
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
        entities: vec![],
        props: vec![
            (MapSize { width: 3, height: 1 }, MapPos { x: 0, y: 0 }),
            (MapSize { width: 3, height: 1 }, MapPos { x: 5, y: 0 }),
        ],
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
        entities: vec![],
        props: vec![
            (MapSize { width: 3, height: 2 }, MapPos { x: 0, y: 0 }),
            (MapSize { width: 3, height: 1 }, MapPos { x: 5, y: 0 }),
            (MapSize { width: 2, height: 1 }, MapPos { x: 5, y: 1 }),
        ],
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
