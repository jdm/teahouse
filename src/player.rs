use bevy::prelude::*;
use crate::entity::{EntityType, Facing, FacingDirection};
use crate::geom::TILE_SIZE;
use crate::tea::Ingredient;
use std::collections::HashMap;

#[derive(Component)]
pub struct Holding {
    pub entity: Entity,
    pub _entity_type: EntityType,
}

#[derive(Component, Default)]
pub struct Player {
    pub carrying: HashMap<Ingredient, u32>,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_system(adjust_held_item);
    }
}

fn adjust_held_item(
    held: Query<&Holding, With<Player>>,
    player: Query<&Facing, With<Player>>,
    mut held_transform: Query<&mut Transform>,
) {
    if held.is_empty() {
        return;
    }
    let facing = player.single();
    let held = held.single();
    let mut transform = held_transform.get_mut(held.entity).unwrap();
    let delta = match (*facing).0 {
        FacingDirection::Up => (0., 1.),
        FacingDirection::Down => (0., -1.),
        FacingDirection::Left => (-1., 0.),
        FacingDirection::Right => (1., 0.),
    };
    transform.translation = Vec2::new(delta.0 * TILE_SIZE, delta.1 * TILE_SIZE)
        .extend(transform.translation.z);
}
