use bevy::prelude::*;
use crate::entity::Paused;
use crate::geom::{
    MapPos, MapSize, HasSize, transform_to_map_pos, map_to_screen, screen_to_map_pos_inner
};
use crate::map::Map;

pub struct MovablePlugin;

impl Plugin for MovablePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(move_movables);
    }
}

// Set up an entity to move from its current screen-space position to the screen-space point
// corresponding to the provided map point. This is used for subtile movement to ensure smooth
// transitions between tiles.
pub fn move_to_screen_point(transform: &Transform, movable: &mut Movable, next: MapPos, sized: &HasSize, map: &Map) {
    let next = map_to_screen(&next, &sized.size, &map);

    let delta = next.x - transform.translation.x;
    let mut subtile_max = Vec2::new(delta, 0.);
    let speed = movable.entity_speed;
    if next.x < transform.translation.x {
        movable.speed.x = -speed;
        movable.direction = MoveDirection::Left;
    } else if next.x > transform.translation.x {
        movable.speed.x = speed;
        movable.direction = MoveDirection::Right;
    } else {
        movable.speed.x = 0.;
    }

    // Screenspace is subtly different than map space when determining which way
    // to move and how much to clamp by. Up is >> 0, down is << 0.
    let delta = (next.y - transform.translation.y).abs();
    let speed = movable.entity_speed;
    if next.y > transform.translation.y {
        movable.speed.y = speed;
        movable.direction = MoveDirection::Up;
        subtile_max.y = delta;
    } else if next.y < transform.translation.y {
        movable.speed.y = -speed;
        movable.direction = MoveDirection::Down;
        subtile_max.y = -delta;
    } else {
        movable.speed.y = 0.;
    }

    // We know how far this entity is from the tile boundary of the tile that it is
    // currently in, so we should not attempt to move farther than that distance.
    movable.subtile_max = Some(subtile_max);
}

pub fn move_to_point(movable: &mut Movable, current: MapPos, next: MapPos) {
    let speed = movable.entity_speed;
    movable.subtile_max = None;
    if next.x < current.x {
        movable.speed.x = -speed;
        movable.direction = MoveDirection::Left;
    } else if next.x > current.x {
        movable.speed.x = speed;
        movable.direction = MoveDirection::Right;
    } else {
        movable.speed.x = 0.;
    }

    if next.y < current.y {
        movable.speed.y = speed;
        movable.direction = MoveDirection::Up;
    } else if next.y > current.y {
        movable.speed.y = -speed;
        movable.direction = MoveDirection::Down;
    } else {
        movable.speed.y = 0.
    }
}

pub fn is_tile_aligned(
    transform: &Transform,
    map: &Map,
    sized: &HasSize,
) -> bool {
    let current_point = transform_to_map_pos(&transform, &map, &sized.size);
    let ideal_point = map_to_screen(&current_point, &sized.size, &map);
    transform.translation == Vec3::new(ideal_point.x, ideal_point.y, transform.translation.z)
}

pub fn reset_movable_pos(transform: &mut Transform, movable: &mut Movable, sized: &HasSize, map: &Map, pos: MapPos) {
    let ideal_point = map_to_screen(&pos, &sized.size, &map);
    transform.translation = Vec3::new(ideal_point.x, ideal_point.y, transform.translation.z);
    movable.subtile_max = None;
    movable.speed = Vec2::ZERO;
}

pub enum MoveDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Component)]
pub struct Movable {
    pub speed: Vec2,
    pub size: Vec2,
    pub entity_speed: f32,
    pub direction: MoveDirection,
    // When present, used to clamp any movement in one frame so it does not
    // exceed the vector given.
    pub subtile_max: Option<Vec2>,
}

impl MoveDirection {
    pub fn anim_index(&self) -> usize {
        match self {
            MoveDirection::Down => 0,
            MoveDirection::Right => 1,
            MoveDirection::Up => 2,
            MoveDirection::Left => 3,
        }
    }
}

struct MovingEntity {
    speed: Vec2,
    translation: Vec3,
    size: MapSize,
}

fn entities_will_collide(first: &MovingEntity, second: &MovingEntity, time_delta: f32, map_size: &MapSize) -> bool {
    let entities_intersecting =
        |first_point: &MapPos, first_size: &MapSize, second_point: &MapPos, second_size: &MapSize| {
            first_point.x + first_size.width > second_point.x &&
                first_point.x < second_point.x + second_size.width &&
                first_point.y + first_size.height > second_point.y &&
                first_point.y < second_point.y + second_size.height
        };

    let base_pos1 = screen_to_map_pos_inner(first.translation.x, first.translation.y, &map_size, &first.size);
    let base_pos2 = screen_to_map_pos_inner(second.translation.x, second.translation.y, &map_size, &second.size);
    if base_pos1 == base_pos2 {
        // Bail out; the entities are already sharing a tile.
        debug!("bailing out ({:?} and {:?})", first.translation, second.translation);
        return false;
    }

    let x1_delta = Vec3::new(first.speed.x, 0., 0.);
    let adjusted1 = first.translation + x1_delta * time_delta;
    let moving_tile_pos = screen_to_map_pos_inner(adjusted1.x, adjusted1.y, &map_size, &first.size);

    let x2_delta = Vec3::new(second.speed.x, 0., 0.);
    let adjusted2 = second.translation + x2_delta * time_delta;
    let fixed_tile_pos = screen_to_map_pos_inner(adjusted2.x, adjusted2.y, &map_size, &second.size);

    let x_colliding = entities_intersecting(&moving_tile_pos, &first.size, &fixed_tile_pos, &second.size);
    if x_colliding {
        debug!("x colliding at {:?} ({:?}) and {:?} ({:?})", moving_tile_pos, adjusted1, fixed_tile_pos, adjusted2);
    }

    let y1_delta = Vec3::new(0., first.speed.y, 0.);
    let adjusted1 = first.translation + y1_delta * time_delta;
    let moving_tile_pos = screen_to_map_pos_inner(adjusted1.x, adjusted1.y, &map_size, &first.size);

    let y2_delta = Vec3::new(0., second.speed.y, 0.);
    let adjusted2 = second.translation + y2_delta * time_delta;
    let fixed_tile_pos = screen_to_map_pos_inner(adjusted2.x, adjusted2.y, &map_size, &second.size);

    let y_colliding = entities_intersecting(&moving_tile_pos, &first.size, &fixed_tile_pos, &second.size);
    if y_colliding {
        debug!("y colliding at {:?} ({:?}) and {:?} ({:?})", moving_tile_pos, adjusted1, fixed_tile_pos, adjusted2);
    }

    x_colliding || y_colliding
}

pub fn move_movables(
    mut set: ParamSet<(
        Query<(Entity, &Movable, &Transform, &HasSize)>,
        Query<&mut Movable>,
        Query<(&Movable, &mut Transform), Without<Paused>>,
    )>,
    timer: Res<Time>,
    map: Res<Map>,
) {
    let delta_seconds = timer.delta_seconds();

    let mut colliding_entities = vec![];
    let q = set.p0();
    for (moving, movable, transform, sized) in &q {
        for (fixed, _movable2, transform2, sized2) in &q {
            if moving == fixed {
                continue;
            }

            // If one of the entities is not moving, the collision check will still take
            // place from the perspective of the entity that _is_ moving. If neither entity
            // is moving, we can skip the check entirely.
            if movable.speed == Vec2::ZERO {
                continue;
            }

            let first = MovingEntity {
                speed: movable.speed,
                translation: transform.translation,
                size: sized.size,
            };
            let second = MovingEntity {
                speed: Vec2::ZERO,
                translation: transform2.translation,
                size: sized2.size,
            };
            let map_size = MapSize { width: map.width, height: map.height };
            let colliding = entities_will_collide(&first, &second, delta_seconds, &map_size);

            if colliding {
                debug!("collision for {:?} and {:?}", moving, fixed);
                colliding_entities.push(moving);
            }
        }
    }

    for entity in colliding_entities {
        let mut q = set.p1();
        let mut movable = q.get_mut(entity).unwrap();
        debug!("resetting speed for {:?}", entity);
        movable.speed = Vec2::ZERO;
    }

    let mut q = set.p2();
    for (movable, mut transform) in &mut q {
        let mut delta = Vec3::new(movable.speed.x, movable.speed.y, 0.0) * delta_seconds;
        if let Some(subtile_max) = movable.subtile_max {
            // When there is an explicit subtile clamp, we need to modify our delta to
            // clamp to whichever value is closer to zero.
            if delta.x > 0. {
                delta.x = delta.x.min(subtile_max.x);
            } else {
                delta.x = delta.x.max(subtile_max.x);
            }
            if delta.y > 0. {
                delta.y = delta.y.min(subtile_max.y);
            } else {
                delta.y = delta.y.max(subtile_max.y);
            }
        }
        transform.translation += delta;
    }
}
