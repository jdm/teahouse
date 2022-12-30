use bevy::prelude::*;
use crate::geom::*;
use crate::map::Map;

pub fn move_to_point(movable: &mut Movable, current: MapPos, next: MapPos) {
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

pub fn reset_movable_pos(transform: &mut Transform, movable: &mut Movable, sized: &HasSize, map: &Map, pos: MapPos) {
    let ideal_point = map_to_screen(&pos, &sized.size, &map);
    transform.translation = Vec3::new(ideal_point.x, ideal_point.y, transform.translation.z);
    movable.speed = Vec2::ZERO;
}

#[derive(Component)]
pub struct Movable {
    pub speed: Vec2,
    pub size: Vec2,
    pub entity_speed: f32,
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
        Query<(&Movable, &mut Transform)>,
    )>,
    timer: Res<Time>,
    map: Res<Map>,
) {
    let delta_seconds = timer.delta_seconds();

    let mut colliding_entities = vec![];
    let q = set.p0();
    for (moving, movable, transform, sized) in &q {
        for (fixed, movable2, transform2, sized2) in &q {
            if moving == fixed {
                continue;
            }

            if movable.speed == Vec2::ZERO && movable2.speed == Vec2::ZERO {
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
        let delta = Vec3::new(movable.speed.x, movable.speed.y, 0.0);
        transform.translation += delta * delta_seconds;
    }
}
