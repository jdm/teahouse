use bevy::prelude::*;
use crate::movable::Movable;

#[derive(Resource)]
pub struct TextureResources {
    pub atlas: Handle<TextureAtlas>,
    pub cycle_length: usize,
}

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

pub fn animate_sprite(
    time: Res<Time>,
    texture_resources: Res<TextureResources>,
    mut query: Query<(
        &mut AnimationTimer,
        &mut TextureAtlasSprite,
        &Movable,
    )>,
) {
    for (mut timer, mut sprite, movable) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            let num_textures = texture_resources.cycle_length;
            sprite.index = movable.direction.anim_index() * num_textures +
                (sprite.index + 1) % num_textures;
        }
    }
}
