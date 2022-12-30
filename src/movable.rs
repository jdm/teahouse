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
    transform.translation = Vec2::new(ideal_point.x, ideal_point.y).extend(0.);
    movable.speed = Vec2::ZERO;
}

#[derive(Component)]
pub struct Movable {
    pub speed: Vec2,
    pub size: Vec2,
    pub entity_speed: f32,
}

pub struct CollisionEvent {
    moving: Entity,
    _fixed: Entity,
}

pub fn check_for_collisions(
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

pub fn halt_collisions(
    mut collision_events: EventReader<CollisionEvent>,
    mut q: Query<&mut Movable>,
) {
    for ev in collision_events.iter() {
        if let Ok(mut movable) = q.get_mut(ev.moving) {
            movable.speed = Vec2::ZERO;
        }
    }
    collision_events.clear();
}

pub fn move_movable(mut q: Query<(&Movable, &mut Transform)>, timer: Res<Time>) {
    for (movable, mut transform) in &mut q {
        let delta = Vec3::new(movable.speed.x, movable.speed.y, 0.0);
        transform.translation += delta * timer.delta_seconds();
    }
}
