use basic_pathfinding::coord::Coord;
use basic_pathfinding::grid::{Grid, GridType};
use basic_pathfinding::pathfinding::find_path as base_find_path;
use basic_pathfinding::pathfinding::SearchOpts;
use bevy::prelude::*;
use bevy::sprite::collide_aabb::collide;
use bevy::time::FixedTimestep;
use rand::seq::IteratorRandom;
use rand_derive2::RandGen;
use std::collections::HashMap;
use std::default::Default;
use std::time::Duration;

// Defines the amount of time that should elapse between each physics step.
const TIME_STEP: f32 = 1.0 / 60.0;

fn main() {
    let map = read_map(MAP);

    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(update_pathing_grid)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                .with_system(check_for_collisions)
                .with_system(halt_collisions.after(check_for_collisions))
                .with_system(move_movable.after(halt_collisions))

        )
        .insert_resource(map)
        .add_system(highlight_interactable)
        .add_system(select_pathfinding_targets)
        .add_system(pathfind.after(update_pathing_grid))
        .add_system(pathfind_to_target.after(update_pathing_grid).before(check_for_collisions))
        .add_system(keyboard_input)
        .add_system(run_cat)
        .add_system(debug_keys)
        .add_system(bevy::window::close_on_esc)
        .add_event::<CollisionEvent>()
        .add_event::<PathfindEvent>()
        .init_resource::<PathingGrid>()
        .run();
}

#[derive(Resource, Default)]
struct PathingGrid {
    grid: Grid,
}

fn update_pathing_grid(
    entities: Query<(&Movable, &Transform, &HasSize)>,
    map: Res<Map>,
    mut grid: ResMut<PathingGrid>,
) {
    let mut tiles = vec![vec![1; map.width]; map.height];
    // We include Movable even though it's unused to only chart the position of physical
    // objects that block walking.
    for (_movable, transform, sized) in &entities {
        let point = transform_to_map_pos(&transform, &map, &sized.size);
        for y in 0..sized.size.height {
            for x in 0..sized.size.width {
                tiles[point.y + y][point.x + x] = 0;
            }
        }
    }

    /*for line in &tiles {
        println!("{:?}", line);
    }*/
    grid.grid = Grid {
        tiles,
        walkable_tiles: vec![1],
        grid_type: GridType::Cardinal,
        ..default()
    };
}

#[derive(Hash, RandGen, Copy, Clone, PartialEq, Eq, Debug)]
enum Ingredient {
    BlackTea,
    OolongTea,
    Chai,
    CitrusPeel,
    MintLeaf,
    Sugar,
    Honey,
    Milk,
    Lemon,
}

struct CollisionEvent {
    moving: Entity,
    _fixed: Entity,
}

#[derive(Component, Debug)]
struct PathfindTarget {
    target: Entity,
    next_point: Option<MapPos>,
    current_goal: MapPos,
    exact: bool,
}

#[derive(Component)]
struct HasSize {
    size: MapSize,
}

#[derive(Component)]
struct Movable {
    speed: Vec2,
    size: Vec2,
    entity_speed: f32,
}

#[derive(Component, Default)]
struct Player {
    carrying: HashMap<Ingredient, u32>,
}

#[derive(Component, Default)]
struct TeaPot {
    ingredients: HashMap<Ingredient, u32>,
    steeped_for: Option<Duration>,
}

#[derive(PartialEq)]
enum CustomerState {
    LookingForChair,
    SittingInChair,
}

#[derive(Debug)]
enum CatState {
    Sleeping(Timer),
    MovingToEntity,
    MovingToBed,
}

#[derive(Component)]
struct Cat {
    state: CatState,
}

impl Default for Cat {
    fn default() -> Self {
        Self {
            state: CatState::Sleeping(Timer::new(Duration::from_secs(2), TimerMode::Once))
        }
    }
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
struct StatusMessage {
    source: Option<Entity>,
}

#[derive(Bundle)]
struct StatusMessageBundle {
    message: StatusMessage,

    #[bundle]
    text: TextBundle,
}

#[derive(Component)]
struct Interactable {
    highlight: Color,
    previous: Option<Color>,
    colliding: bool,
    message: String,
}

impl Default for Interactable {
    fn default() -> Self {
        Self {
            highlight: Color::BLACK,
            previous: None,
            colliding: false,
            message: String::new(),
        }
    }
}

#[derive(Component)]
struct Prop;

#[derive(Component)]
struct TeaStash {
    ingredient: Ingredient,
    amount: u32,
}

#[derive(Component)]
struct Stove;

#[derive(Component)]
struct Door;

#[derive(Component)]
struct CatBed;

#[derive(Component)]
struct Chair {
    pos: MapPos,
    occupied: bool,
}

#[derive(Component)]
struct Cupboard {
    teapots: u32,
}

#[derive(Component)]
struct DebugTile;

struct PathfindEvent {
    customer: Entity,
    destination: MapPos,
}

fn pathfind_to_target(
    mut set: ParamSet<(
        Query<&PathfindTarget>,
        Query<(Entity, &Transform, &HasSize)>,
        Query<(Entity, &mut PathfindTarget, &mut Transform, &mut Movable, &HasSize)>,
    )>,
    map: Res<Map>,
    mut commands: Commands,
    grid: Res<PathingGrid>,
    debug_tile: Query<(Entity, &DebugTile, &mut Sprite)>,
) {
    let mut target_entities = vec![];
    for target in &set.p0() {
        target_entities.push(target.target);
    }

    let mut target_data = HashMap::new();
    for (entity, transform, sized) in &set.p1() {
        if !target_entities.contains(&entity) || target_data.contains_key(&entity) {
            continue;
        }
        let target_point = transform_to_map_pos(&transform, &map, &sized.size);
        target_data.insert(entity, target_point);
    }

    for (entity, mut target, mut transform, mut movable, sized) in &mut set.p2() {
        let current_point = transform_to_map_pos(&transform, &map, &sized.size);
        if target.next_point.map_or(true, |point| current_point == point) {
            reset_movable_pos(&mut transform, &mut movable, &sized, &map, current_point);

            let target_point = target_data[&target.target];
            // FIXME: is this necessary, or can we rely on an empty path instead?
            if target_point == current_point || current_point == target.current_goal {
                commands.entity(entity).remove::<PathfindTarget>();
                continue;
            }

            let path = find_path(&grid, &map, &transform, target_point, target.exact);
            if let Some((path, actual_target_point)) = path {
                // We have reached the goal.
                if path.is_empty() {
                    commands.entity(entity).remove::<PathfindTarget>();
                    continue;
                }

                target.next_point = Some(path[0]);
                target.current_goal = actual_target_point;

                for (debug_entity, _, _) in &debug_tile {
                    commands.entity(debug_entity).despawn();
                }

                for point in path {
                    let next_screen_rect = map_to_screen(&point, &MapSize { width: 1, height: 1 }, &map);
                    let next_screen_point = Vec3::new(next_screen_rect.x, next_screen_rect.y, 0.);
                    commands.spawn((
                        DebugTile,
                        SpriteBundle {
                            sprite: Sprite {
                                color: Color::rgba(0.5, 0., 0., 0.2),
                                custom_size: Some(Vec2::new(
                                    next_screen_rect.w * 0.6,
                                    next_screen_rect.h * 0.6,
                                )),
                                ..default()
                            },
                            transform: Transform::from_translation(next_screen_point),
                            ..default()
                        },
                    ));
                }
            } else {
                debug!("No path to {:?} for {:?}", target.target, entity);
            }
        } else {
            move_to_point(&mut movable, current_point, target.next_point.unwrap());
        }
    }

}

fn run_cat(
    mut cat: Query<(Entity, &mut Cat, Option<&PathfindTarget>, &mut Transform)>,
    cat_bed: Query<(Entity, &CatBed)>,
    player: Query<(Entity, &Player)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let (entity, mut cat, target, mut transform) = cat.single_mut();
    let mut find_entity = false;
    let mut find_bed = false;
    let mut sleep = false;
    //println!("{:?} {:?}", cat.state, target);
    match cat.state {
        CatState::Sleeping(ref mut timer) => {
            timer.tick(time.delta());
            find_entity = timer.finished();
            transform.scale = Vec2::splat(time.elapsed_seconds().sin() + 0.5).extend(0.);
        }
        CatState::MovingToEntity => find_bed = target.is_none(),
        CatState::MovingToBed => sleep = target.is_none(),
    }

    if find_entity {
        transform.scale = Vec2::splat(1.0).extend(0.);
        println!("setting target entity");
        cat.state = CatState::MovingToEntity;
        let (player_entity, _) = player.single();
        commands.entity(entity).insert(PathfindTarget {
            target: player_entity,
            next_point: None,
            exact: false,
            current_goal: MapPos { x: 0, y: 0 },
        });
    }

    if find_bed {
        println!("returning to bed");
        cat.state = CatState::MovingToBed;
        let (cat_bed_entity, _) = cat_bed.single();
        commands.entity(entity).insert(PathfindTarget {
            target: cat_bed_entity,
            next_point: None,
            exact: true,
            current_goal: MapPos { x: 0, y: 0 },
        });
    }

    if sleep {
        println!("going to sleep for 5s");
        cat.state = CatState::Sleeping(Timer::new(Duration::from_secs(5), TimerMode::Once));
    }    
}

fn move_to_point(movable: &mut Movable, current: MapPos, next: MapPos) {
    let speed = movable.entity_speed;
    if next.x < current.x {
        movable.speed.x = -speed;
    } else if next.x > current.x {
        movable.speed.x = speed;
    } else {
        movable.speed.x = 0.;
    }

    if next.y < current.y {
        movable.speed.y = speed;
    } else if next.y > current.y {
        movable.speed.y = -speed;
    } else {
        movable.speed.y = 0.
    }
}

fn reset_movable_pos(transform: &mut Transform, movable: &mut Movable, sized: &HasSize, map: &Map, pos: MapPos) {
    let ideal_point = map_to_screen(&pos, &sized.size, &map);
    transform.translation = Vec2::new(ideal_point.x, ideal_point.y).extend(0.);
    movable.speed = Vec2::ZERO;
}

fn select_pathfinding_targets(
    mut q: Query<(Entity, &mut Customer, &mut Movable, &mut Transform, &HasSize)>,
    mut chairs: Query<&mut Chair>,
    mut pathfind_events: EventWriter<PathfindEvent>,
    map: Res<Map>
) {
    for (entity, mut customer, mut movable, mut transform, sized) in &mut q {
        if customer.goal.is_none() && customer.state == CustomerState::LookingForChair {
            // FIXME: if goal fails, should choose another.
            for chair in &chairs {
                if !chair.occupied {
                    debug!("giving customer a goal: {:?}", chair.pos);
                    pathfind_events.send(PathfindEvent {
                        customer: entity,
                        destination: chair.pos,
                    });
                    break;
                }
            }
        } else if let Some(point) = customer.path.as_ref().and_then(|path| path.first().cloned()) {
            let current_point = transform_to_map_pos(&transform, &map, &sized.size);
            debug!("screen point: {:?}, current point: {:?}, next goal: {:?}", transform.translation, current_point, point);
            if current_point == point {
                debug!("reached target point, resetting");
                customer.path.as_mut().unwrap().remove(0);
                reset_movable_pos(&mut transform, &mut movable, &sized, &map, current_point);
            } else {
                move_to_point(&mut movable, current_point, point);
            }
        } else if let Some(goal) = customer.goal.clone() {
            let current_point = transform_to_map_pos(&transform, &map, &sized.size);
            debug!("current point: {:?}, terminal goal: {:?}", current_point, goal);
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

fn find_path(
    grid: &PathingGrid,
    map: &Map,
    from: &Transform,
    to: MapPos,
    exact: bool,
) -> Option<(Vec<MapPos>, MapPos)> {
    // FIXME: assume that only 1x1 entities need pathfinding.
    let start = transform_to_map_pos(from, &map, &MapSize { width: 1, height: 1 });
    let start_grid = Coord::new(start.x as i32, start.y as i32);

    let exact_end = Coord::new(to.x as i32, to.y as i32);
    let end = if exact {
        exact_end
    } else {
        let mut rng = rand::thread_rng();
        let random_adjacent = grid.grid
            .get_adjacent(&exact_end)
            .into_iter()
            .filter(|point| {
                grid.grid.is_coord_walkable(point.x, point.y)
            })
            .choose(&mut rng);
        if random_adjacent.is_none() {
            return None;
        }
        random_adjacent.unwrap()
    };
    let options = SearchOpts {
        path_adjacent: false,
        ..default()
    };
    let path = base_find_path(&grid.grid, start_grid, end, options);
    debug!("path from {:?} to {:?}: {:?}", start, to, path);
    return path
        .map(|path| {
            path.into_iter()
                .map(|point| MapPos { x: point.x as usize, y: point.y as usize })
                .collect()
        })
        .map(|path| (path, MapPos { x: end.x as usize, y: end.y as usize }));
}

fn pathfind(
    mut pathfind_events: EventReader<PathfindEvent>,
    mut q: Query<(Entity, &mut Customer, &Transform)>,
    map: Res<Map>,
    grid: Res<PathingGrid>,
) {
    for ev in pathfind_events.iter() {
        for (entity, mut customer, transform) in &mut q {
            if entity != ev.customer {
                continue;
            }

            let path = find_path(&grid, &map, &transform, ev.destination, true);
            if path.is_none() {
                debug!("no path to goal!");
                continue;
            }
            customer.goal = Some(ev.destination);
            customer.path = Some(path.unwrap().0);
            break;
        }
    }
    pathfind_events.clear();
}

fn check_for_collisions(
    q: Query<(Entity, &Movable, &Transform, &HasSize)>,
    timer: Res<Time>,
    map: Res<Map>,
    mut collision_events: EventWriter<CollisionEvent>,
) {
    for (moving, movable, transform, sized) in &q {
        for (fixed, _movable2, transform2, sized2) in &q {
            if moving == fixed {
                continue;
            }

            if movable.speed == Vec2::ZERO {
                continue;
            }

            let delta = movable.speed.extend(0.);
            let adjusted = transform.translation + delta * timer.delta_seconds();
            let moving_tile_pos = screen_to_map_pos(adjusted.x, adjusted.y, &map, &sized.size);
            let fixed_tile_pos = screen_to_map_pos(transform2.translation.x, transform2.translation.y, &map, &sized2.size);

            //let collision = collide(adjusted, movable.size, transform2.translation, movable2.size);
            //if collision.is_some() {
            let mut colliding = false;
            'exit: for y in 0..sized.size.height {
                for x in 0..sized.size.width {
                    for y2 in 0..sized2.size.height {
                        for x2 in 0..sized2.size.width {
                            if x + moving_tile_pos.x == x2 + fixed_tile_pos.x &&
                                y + moving_tile_pos.y == y2 + fixed_tile_pos.y
                            {
                                colliding = true;
                                break 'exit;
                            }
                        }
                    }
                }
            }
            //if moving_tile_pos == fixed_tile_pos {
            if colliding {
                debug!("collision for {:?} and {:?}", moving, fixed);
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

const CAT_SPEED: f32 = 25.0;
const CUSTOMER_SPEED: f32 = 40.0;
const SPEED: f32 = 150.0;
const TILE_SIZE: f32 = 25.0;

fn highlight_interactable(
    player: Query<(&Transform, &Movable), With<Player>>,
    mut interactable: Query<(Entity, &mut Interactable, &Transform, &mut Sprite, &Movable)>,
    mut status: Query<(&mut StatusMessage, &mut Text)>,
) {
    let (player_transform, player_movable) = player.single();
    let (mut status, mut status_text) = status.single_mut();

    for (entity, mut interactable, transform, mut sprite, movable) in interactable.iter_mut() {
        let collision = collide(
            transform.translation,
            movable.size,
            player_transform.translation,
            player_movable.size * 1.3,
        );
        if collision.is_some() && status.source.is_none() {
            if interactable.previous.is_none() {
                interactable.previous = Some(sprite.color);
                sprite.color = interactable.highlight;
            }
            interactable.colliding = true;

            status.source = Some(entity);
            status_text.sections[0].value = interactable.message.clone();

        } else if collision.is_none() && interactable.previous.is_some() {
            sprite.color = interactable.previous.take().unwrap();
            interactable.colliding = false;

            status.source = None;
            status_text.sections[0].value = "".to_string();
        }
    }
}

fn keyboard_input(
    keys: Res<Input<KeyCode>>,
    mut q: Query<(Entity, &mut Player, &mut Movable)>,
    mut interactables: Query<(&mut TeaStash, &Interactable)>,
    mut cupboards: Query<(&mut Cupboard, &Interactable)>,
    mut commands: Commands,
) {
    let (player_entity, mut player, mut movable) = q.single_mut();

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

    if keys.just_released(KeyCode::X) {
        for (mut stash, interactable) in &mut interactables {
            if interactable.colliding {
                stash.amount -= 1;
                let amount = player.carrying.entry(stash.ingredient).or_insert(0);
                *amount += 1;
                println!("carrying: {:?}", player.carrying);
                return;
            }
        }
        for (mut cupboard, interactable) in &mut cupboards {
            if interactable.colliding {
                cupboard.teapots -= 1;
                commands.entity(player_entity).insert(TeaPot::default());
                println!("acquired teapot");
                return;
            }
        }

        //if player.
    }
}

fn debug_keys(
    keys: Res<Input<KeyCode>>,
    q: Query<(&Transform, &HasSize), With<Door>>,
    mut commands: Commands,
    map: Res<Map>,
) {
    if keys.just_released(KeyCode::C) {
        let (transform, sized) = q.iter().next().unwrap();
        let mut door_pos = transform_to_map_pos(&transform, &map, &sized.size);
        door_pos.x += 1;
        // FIXME: assume customers are all 1x1 entities.
        let screen_rect = map_to_screen(&door_pos, &MapSize { width: 1, height: 1 }, &map);

        spawn_sprite(EntityType::Customer, screen_rect, &mut commands);
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum EntityType {
    Customer,
    Player,
    Prop,
    Chair(MapPos),
    Door,
    Stove,
    TeaStash(Ingredient, u32),
    Cupboard(u32),
    CatBed,
    Cat,
}

fn spawn_sprite(entity: EntityType, rect: ScreenRect, commands: &mut Commands) {
    let pos = Vec2::new(rect.x, rect.y);
    let size = Vec2::new(rect.w, rect.h);
    let speed = match entity {
        EntityType::Player => SPEED,
        EntityType::Customer => CUSTOMER_SPEED,
        EntityType::Cat => CAT_SPEED,
        _ => 0.,
    };
    let color = match entity {
        EntityType::Player => Color::rgb(0.25, 0.25, 0.75),
        EntityType::Customer => Color::rgb(0.0, 0.25, 0.0),
        EntityType::Prop => Color::rgb(0.25, 0.15, 0.0),
        EntityType::Chair(..) => Color::rgb(0.15, 0.05, 0.0),
        EntityType::Door => Color::rgb(0.6, 0.2, 0.2),
        EntityType::Stove => Color::rgb(0.8, 0.8, 0.8),
        EntityType::TeaStash(..) => Color::rgb(0.3, 0.3, 0.3),
        EntityType::Cupboard(..) => Color::rgb(0.5, 0.35, 0.0),
        EntityType::CatBed => Color::rgb(0., 0., 0.25),
        EntityType::Cat => Color::BLACK,
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
    let movable = Movable { speed: Vec2::ZERO, size: size, entity_speed: speed };
    let sized = HasSize {
        size: MapSize {
            width: (rect.w / TILE_SIZE) as usize,
            height: (rect.h / TILE_SIZE) as usize,
        }
    };
    match entity {
        EntityType::Player => {
            commands.spawn((Player::default(), movable, sized, sprite))
                .with_children(|parent| {
                    parent.spawn(Camera2dBundle::default());
                });
        }
        EntityType::Customer => {
            commands.spawn((Customer::default(), movable, sized, sprite));
        }
        EntityType::Cat => {
            commands.spawn((Cat::default(), movable, sized, sprite));
        }
        EntityType::Chair(pos) => {
            commands.spawn((
                Chair {
                    pos,
                    occupied: false,
                },
                sized,
                sprite,
            ));
        }
        EntityType::Prop => {
            commands.spawn((Prop, movable, sized, sprite));
        }
        EntityType::Door => {
            commands.spawn((Door, movable, sized, sprite));
        }
        EntityType::Stove => {
            commands.spawn((
                Stove,
                Interactable {
                    highlight: Color::rgb(1., 1., 1.),
                    message: "Press X to toggle burner".to_string(),
                    ..default()
                },
                movable,
                sized,
                sprite,
            ));
        }
        EntityType::TeaStash(ingredient, amount) => {
            commands.spawn((
                TeaStash { ingredient, amount },
                Interactable {
                    highlight: Color::rgb(1., 1., 1.),
                    message: format!("Press X to pick up {:?}", ingredient),
                    ..default()
                },
                movable,
                sized,
                sprite,
            ));
        }
        EntityType::Cupboard(pots) => {
            commands.spawn((
                Cupboard { teapots: pots },
                Interactable {
                    highlight: Color::rgb(1., 1., 1.),
                    message: "Press X to pick up teapot".to_string(),
                    ..default()
                },
                movable,
                sized,
                sprite,
            ));
        }
        EntityType::CatBed => {
            commands.spawn((
                CatBed,
                sized,
                sprite,
            ));
        }
    };
}

static MAP: &[&str] = &[
    "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxB.xxxxxxxxxxx",
    "xb....................................xsssxxxxx",
    "x.k.............P............................tx",
    "x..........c......................xx.........tx",
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
    doors: Vec<MapPos>,
    stoves: Vec<MapPos>,
    tea_stashes: Vec<MapPos>,
    cupboards: Vec<MapPos>,
    cat_beds: Vec<MapPos>,
    cats: Vec<MapPos>,
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
            } else if ch == 'B' {
                map.cupboards.push(MapPos { x, y });
            } else if ch == 'D' {
                map.doors.push(MapPos { x, y });
            } else if ch == 's' {
                map.stoves.push(MapPos { x, y });
            } else if ch == 't' {
                map.tea_stashes.push(MapPos { x, y });
            } else if ch == 'b' {
                map.cat_beds.push(MapPos { x, y });
            } else if ch == 'k' {
                map.cats.push(MapPos { x, y });
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

fn screen_to_map_pos(x: f32, y: f32, map: &Map, size: &MapSize) -> MapPos {
    let x = (x - size.width as f32 * TILE_SIZE / 2.) / TILE_SIZE + map.width as f32 / 2.;
    let y = -((y + size.height as f32 * TILE_SIZE / 2.) / TILE_SIZE - map.height as f32 / 2.);
    assert!(x >= 0.);
    assert!(y >= 0.);
    MapPos { x: x as usize, y: y as usize }
}

fn transform_to_map_pos(transform: &Transform, map: &Map, size: &MapSize) -> MapPos {
    let translation = transform.translation;
    screen_to_map_pos(translation.x, translation.y, map, size)
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
    asset_server: Res<AssetServer>,
) {
    for pos in &map.chairs {
        let rect = map_to_screen(pos, &MapSize { width: 1, height: 1 }, &map);
        spawn_sprite(
            EntityType::Chair(*pos),
            rect,
            &mut commands,
        )
    }

    for pos in &map.cat_beds {
        let rect = map_to_screen(pos, &MapSize { width: 2, height: 2 }, &map);
        spawn_sprite(
            EntityType::CatBed,
            rect,
            &mut commands,
        )
    }

    for pos in &map.cat_beds {
        let rect = map_to_screen(pos, &MapSize { width: 1, height: 1 }, &map);
        spawn_sprite(
            EntityType::Cat,
            rect,
            &mut commands,
        )
    }

    for pos in &map.cupboards {
        let rect = map_to_screen(pos, &MapSize { width: 2, height: 1 }, &map);
        spawn_sprite(
            EntityType::Cupboard(rand::random()),
            rect,
            &mut commands,
        )
    }

    for pos in &map.doors {
        let rect = map_to_screen(pos, &MapSize { width: 1, height: 1 }, &map);
        spawn_sprite(
            EntityType::Door,
            rect,
            &mut commands,
        )
    }

    for pos in &map.stoves {
        let rect = map_to_screen(pos, &MapSize { width: 1, height: 1 }, &map);
        spawn_sprite(
            EntityType::Stove,
            rect,
            &mut commands,
        )
    }

    for pos in &map.tea_stashes {
        let rect = map_to_screen(pos, &MapSize { width: 1, height: 1 }, &map);
        spawn_sprite(
            EntityType::TeaStash(Ingredient::generate_random(), rand::random()),
            rect,
            &mut commands,
        )
    }

    for (size, pos) in &map.props {
        let rect = map_to_screen(pos, size, &map);
        spawn_sprite(
            EntityType::Prop,
            rect,
            &mut commands,
        )
    }

    for (entity_type, pos) in &map.entities {
        let size = MapSize { width: 1, height: 1 };
        let rect = map_to_screen(pos, &size, &map);
        spawn_sprite(
            *entity_type,
            rect,
            &mut commands,
        );
    }

    commands.spawn(
        StatusMessageBundle {
            message: StatusMessage {
                source: None,
            },
            text: TextBundle::from_section(
                "",
                TextStyle {
                    font: asset_server.load("Lato-Medium.ttf"),
                    font_size: 25.0,
                    color: Color::WHITE,
                },
            )
                .with_text_alignment(TextAlignment::TOP_CENTER)
                .with_style(Style {
                    position_type: PositionType::Absolute,
                    position: UiRect {
                        bottom: Val::Px(5.0),
                        right: Val::Px(15.0),
                        ..default()
                    },
                    ..default()
                }),
        }
    );
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
    let size = MapSize { width: 1, height: 1 };
    let mut screen_coords = map_to_screen(
        &MapPos { x: 4, y: 4 },
        &size,
        &map
    );
    for _ in 0..TILE_SIZE as usize {
        assert_eq!(
            screen_to_map_pos(screen_coords.x, screen_coords.y, &map, &size),
            MapPos { x: 4, y: 4 },
        );
        screen_coords.x += 1.;
    }

    assert_eq!(
        screen_to_map_pos(screen_coords.x, screen_coords.y, &map, &size),
        MapPos { x: 5, y: 4 },
    );
}

#[test]
fn screen_to_map_subtile_y() {
    let map = Map { width: 8, height: 8, ..default() };
    let size = MapSize { width: 1, height: 1 };
    let mut screen_coords = map_to_screen(
        &MapPos { x: 4, y: 4 },
        &size,
        &map
    );
    for _ in 0..TILE_SIZE as usize {
        assert_eq!(
            screen_to_map_pos(screen_coords.x, screen_coords.y, &map, &size),
            MapPos { x: 4, y: 4 },
        );
        screen_coords.y -= 1.;
    }

    assert_eq!(
        screen_to_map_pos(screen_coords.x, screen_coords.y, &map, &size),
        MapPos { x: 4, y: 5 },
    );
}

#[test]
fn screen_to_map_x_wide() {
    let map = Map { width: 8, height: 8, ..default() };
    let size = MapSize { width: 8, height: 1 };
    let screen_coords = map_to_screen(
        &MapPos { x: 4, y: 4 },
        &size,
        &map
    );
    assert_eq!(
        screen_to_map_pos(screen_coords.x, screen_coords.y, &map, &size),
        MapPos { x: 4, y: 4 },
    );
}

#[test]
fn screen_to_map_y_tall() {
    let map = Map { width: 8, height: 8, ..default() };
    let size = MapSize { width: 1, height: 8 };
    let screen_coords = map_to_screen(
        &MapPos { x: 4, y: 4 },
        &size,
        &map
    );
    assert_eq!(
        screen_to_map_pos(screen_coords.x, screen_coords.y, &map, &size),
        MapPos { x: 4, y: 4 },
    );
}
