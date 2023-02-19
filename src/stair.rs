use bevy::prelude::*;
use crate::entity::Facing;
use crate::geom::{HasSize, transform_to_map_pos, map_to_screen, TILE_SIZE};
use crate::interaction::Interactable;
use crate::map::Map;
use crate::movable::Movable;
use crate::player::Player;

pub struct StairPlugin;

impl Plugin for StairPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_system(stair_collide);
    }
}

#[derive(Component)]
struct Staircase {
    id: String,
    destination: String,
}

pub fn spawn_staircase(
    commands: &mut Commands,
    _movable: Movable,
    sized: HasSize,
    transform: Transform,
    id: String,
    destination: String,
) {
    commands.spawn((
        Staircase { id, destination },
        //movable,
        sized,
        transform,
    ));
}

fn stair_collide(
    all_stairs: Query<(&Staircase, &Transform, &HasSize), Without<Player>>,
    //touching_stairs: Query<&Staircase>,
    mut player: Query<(&Movable, &mut Transform, &Facing), With<Player>>,
    map: Res<Map>,
) {
    if player.is_empty() {
        return;
    }
    let (player, mut player_transform, facing) = player.single_mut();
    if player.speed.x == 0.0 {
        return;
    }
    for (stair, transform, size) in &all_stairs {
        let colliding = bevy::sprite::collide_aabb::collide(
            transform.translation,
            Vec2::splat(TILE_SIZE),
            player_transform.translation,
            Vec2::splat(TILE_SIZE),
        ).is_some();
        if !colliding {
            continue;
        }

        let dest = &stair.destination;
        for (dest_stair, dest_transform, dest_size) in &all_stairs {
            if dest_stair.id != *dest {
                continue;
            }
            let dest_map_pos = transform_to_map_pos(dest_transform, &map, &dest_size.size);
            let adjusted = facing.adjust_pos(&dest_map_pos);
            let adjusted_screen = map_to_screen(&adjusted, &dest_size.size, &map);
            let adjusted_screen = Vec2::new(adjusted_screen.x, adjusted_screen.y);
            player_transform.translation = adjusted_screen.extend(player_transform.translation.z);
            break;
        }
    }
}
