use bevy::prelude::*;
use basic_pathfinding::coord::Coord;
use basic_pathfinding::grid::{Grid, GridType};
use basic_pathfinding::pathfinding::find_path as base_find_path;
use basic_pathfinding::pathfinding::SearchOpts;
use crate::debug::{DebugTile, DebugSettings, create_debug_path};
use crate::entity::Facing;
use crate::geom::{HasSize, MapPos, MapSize, transform_to_map_pos};
use crate::movable::{
    Movable, move_to_point, is_tile_aligned, move_to_screen_point, reset_movable_pos, move_movables
};
use crate::map::Map;
use rand::seq::IteratorRandom;
use std::collections::HashMap;
use std::default::Default;
use std::ops::DerefMut;

pub struct PathfindingPlugin;

impl Plugin for PathfindingPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_system(update_pathing_grid)
            .add_system(pathfind_to_target.after(update_pathing_grid).before(move_movables))
            .init_resource::<PathingGrid>();
    }
}

#[derive(Resource, Default)]
pub struct PathingGrid {
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

type Callback = fn(Entity, &mut Commands);

#[derive(Component)]
struct StopOnCurrentTile(Callback);

// Interrupt the pathfinding for this entity once it reaches the next tile.
// Run a callback when this occurs, to support operations like transitioning the
// entity into a new state.
pub fn stop_current_pathfinding(entity: Entity, commands: &mut Commands, update: Callback) {
    commands.entity(entity).insert(StopOnCurrentTile(update));
}

fn pathfind_to_target(
    mut set: ParamSet<(
        Query<&PathfindTarget>,
        Query<(&Transform, &HasSize)>,
        Query<(Entity, &mut PathfindTarget, &mut Transform, &mut Movable, Option<&mut Facing>, &HasSize, Option<&StopOnCurrentTile>)>,
    )>,
    map: Res<Map>,
    mut commands: Commands,
    grid: Res<PathingGrid>,
    debug_tile: Query<(Entity, &DebugTile, &mut Sprite)>,
    debug_settings: Res<DebugSettings>,
) {
    let mut target_entities = vec![];
    for target in &set.p0() {
        target_entities.push(target.target);
    }

    let q = set.p1();
    let mut target_data = HashMap::new();
    for target_entity in &target_entities {
        if target_data.contains_key(&target_entity) {
            continue;
        }
        if let Ok((transform, sized)) = q.get(*target_entity) {
            let target_point = transform_to_map_pos(&transform, &map, &sized.size);
            target_data.insert(target_entity, target_point);
        }
    }

    for (entity, mut target, mut transform, mut movable, mut facing, sized, will_stop) in &mut set.p2() {
        let current_point = transform_to_map_pos(&transform, &map, &sized.size);
        if target.next_point.map_or(true, |point| current_point == point) {
            // We're within the right tile, but still need to move to the right subtile coordinates.
            if !is_tile_aligned(&transform, &map, &sized) {
                move_to_screen_point(
                    &transform,
                    &mut movable,
                    facing.as_mut().map(DerefMut::deref_mut),
                    target.next_point.unwrap(),
                    &sized,
                    &map,
                );
                continue;
            }

            reset_movable_pos(&mut transform, &mut movable, &sized, &map, current_point);

            if let Some(will_stop) = will_stop {
                commands.entity(entity).remove::<PathfindTarget>();
                commands.entity(entity).remove::<StopOnCurrentTile>();
                (will_stop.0)(entity, &mut commands);
                continue;
            }

            for (debug_entity, debug_tile, _) in &debug_tile {
                if debug_tile.for_entity == entity {
                    commands.entity(debug_entity).despawn();
                }
            }

            let target_point = match target_data.get(&target.target) {
                Some(point) => *point,
                None => {
                    warn!("Target {:?} no longer exists; giving up.", target.target);
                    commands.entity(entity).remove::<PathfindTarget>();
                    continue;
                }
            };

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

                if debug_settings.show_paths {
                    create_debug_path(entity, &path, &map, &mut commands);
                }
            } else {
                warn!("No path to {:?} for {:?}", target.target, entity);
                // If we're in the middle of making a path to a target, resume next cycle.
                // Otherwise, give up and remove any trace of the pathing attempt.
                if target.next_point.is_none() {
                    commands.entity(entity).remove::<PathfindTarget>();
                }
            }
        } else {
            move_to_point(
                &mut movable,
                facing.as_mut().map(DerefMut::deref_mut),
                current_point,
                target.next_point.unwrap(),
            );
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
