use bevy::prelude::*;
use basic_pathfinding::coord::Coord;
use basic_pathfinding::grid::{Grid, GridType};
use basic_pathfinding::pathfinding::find_path as base_find_path;
use basic_pathfinding::pathfinding::SearchOpts;
use crate::debug::*;
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
    target: Entity,
    next_point: Option<MapPos>,
    current_goal: MapPos,
    exact: bool,
}

impl PathfindTarget {
    pub fn new(target: Entity, exact: bool) -> Self {
        Self {
            target,
            next_point: None,
            current_goal: MapPos { x: 0, y: 0 },
            exact
        }
    }
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

                for (debug_entity, debug_tile, _) in &debug_tile {
                    if debug_tile.for_entity == entity {
                        commands.entity(debug_entity).despawn();
                    }
                }

                for point in path {
                    let next_screen_rect = map_to_screen(&point, &MapSize { width: 1, height: 1 }, &map);
                    let next_screen_point = Vec3::new(next_screen_rect.x, next_screen_rect.y, 0.);
                    commands.spawn((
                        DebugTile { for_entity: entity },
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
