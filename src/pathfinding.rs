use bevy::prelude::*;
use basic_pathfinding::coord::Coord;
use basic_pathfinding::grid::{Grid, GridType};
use basic_pathfinding::pathfinding::find_path as base_find_path;
use basic_pathfinding::pathfinding::SearchOpts;
use crate::debug::*;
use crate::entity::{Customer, CustomerState, Chair};
use crate::geom::*;
use crate::movable::*;
use crate::map::Map;
use rand::seq::IteratorRandom;
use std::collections::HashMap;
use std::default::Default;

#[derive(Resource, Default)]
pub struct PathingGrid {
    grid: Grid,
}

pub fn update_pathing_grid(
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

#[derive(Component, Debug)]
pub struct PathfindTarget {
    pub target: Entity,
    pub next_point: Option<MapPos>,
    pub current_goal: MapPos,
    pub exact: bool,
}

pub fn pathfind_to_target(
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

pub fn pathfind(
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

pub struct PathfindEvent {
    customer: Entity,
    destination: MapPos,
}

pub fn select_pathfinding_targets(
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
