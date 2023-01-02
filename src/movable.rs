use bevy::prelude::*;
use crate::entity::{Facing, FacingDirection, Paused};
use crate::geom::{
    MapPos, MapSize, HasSize, transform_to_map_pos, map_to_screen,
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
pub fn move_to_screen_point(
    transform: &Transform,
    movable: &mut Movable,
    facing: Option<&mut Facing>,
    next: MapPos,
    sized: &HasSize,
    map: &Map,
) {
    let next = map_to_screen(&next, &sized.size, &map);

    let delta = next.x - transform.translation.x;
    let mut subtile_max = Vec2::new(delta, 0.);
    let speed = movable.entity_speed;
    let mut direction = None;
    if next.x < transform.translation.x {
        movable.speed.x = -speed;
        direction = Some(FacingDirection::Left);
    } else if next.x > transform.translation.x {
        movable.speed.x = speed;
        direction = Some(FacingDirection::Right);
    } else {
        movable.speed.x = 0.;
    }

    // Screenspace is subtly different than map space when determining which way
    // to move and how much to clamp by. Up is >> 0, down is << 0.
    let delta = (next.y - transform.translation.y).abs();
    let speed = movable.entity_speed;
    if next.y > transform.translation.y {
        movable.speed.y = speed;
        direction = Some(FacingDirection::Up);
        subtile_max.y = delta;
    } else if next.y < transform.translation.y {
        movable.speed.y = -speed;
        direction = Some(FacingDirection::Down);
        subtile_max.y = -delta;
    } else {
        movable.speed.y = 0.;
    }

    if let (Some(facing), Some(direction)) = (facing, direction) {
        facing.0 = direction;
    }

    // We know how far this entity is from the tile boundary of the tile that it is
    // currently in, so we should not attempt to move farther than that distance.
    movable.subtile_max = Some(subtile_max);
}

pub fn move_to_point(
    movable: &mut Movable,
    facing: Option<&mut Facing>,
    current: MapPos,
    next: MapPos,
) {
    let speed = movable.entity_speed;
    movable.subtile_max = None;
    let mut direction = None;
    if next.x < current.x {
        movable.speed.x = -speed;
        direction = Some(FacingDirection::Left);
    } else if next.x > current.x {
        movable.speed.x = speed;
        direction = Some(FacingDirection::Right);
    } else {
        movable.speed.x = 0.;
    }

    if next.y < current.y {
        movable.speed.y = speed;
        direction = Some(FacingDirection::Up);
    } else if next.y > current.y {
        movable.speed.y = -speed;
        direction = Some(FacingDirection::Down);
    } else {
        movable.speed.y = 0.
    }

    if let (Some(facing), Some(direction)) = (facing, direction) {
        facing.0 = direction;
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


#[derive(Component, Default)]
pub struct Movable {
    pub speed: Vec2,
    pub size: Vec2,
    pub entity_speed: f32,
    // When present, used to clamp any movement in one frame so it does not
    // exceed the vector given.
    pub subtile_max: Option<Vec2>,
}

pub fn move_movables(
    mut set: ParamSet<(
        Query<(Entity, &Movable, &Transform)>,
        Query<&mut Movable>,
        Query<(&Movable, &mut Transform), Without<Paused>>,
    )>,
    timer: Res<Time>,
    map: Res<Map>,
) {
    let delta_seconds = timer.delta_seconds();

    let mut colliding_entities = vec![];
    let q = set.p0();
    for (moving, movable, transform) in &q {
        // If one of the entities is not moving, the collision check will still take
        // place from the perspective of the entity that _is_ moving. If neither entity
        // is moving, we can skip the check entirely.
        if movable.speed == Vec2::ZERO {
            continue;
        }

        for (fixed, movable2, transform2) in &q {
            if moving == fixed {
                continue;
            }

            // FIXME: first check X, then check Y, and reset speed independently.
            let colliding = bevy::sprite::collide_aabb::collide(
                transform.translation + movable.speed.extend(0.) * delta_seconds,
                movable.size * 0.9,
                transform2.translation + movable2.speed.extend(0.) * delta_seconds,
                movable2.size * 0.9,
            ).is_some();

            if colliding {
                debug!("collision for {:?} and {:?}", moving, fixed);
                colliding_entities.push(moving);
                break;
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
