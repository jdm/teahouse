use bevy::prelude::*;
use std::collections::HashMap;
use std::default::Default;

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<AtlasAnimationData>()
            .add_system(update_animation_timer)
            .add_system(animate_sprite);
    }
}

pub struct AnimData {
    pub index: usize,
    pub frames: usize,
    pub delay: f32,
}

#[derive(Resource, Default)]
pub struct AtlasAnimationData {
    pub data: HashMap<Handle<TextureAtlas>, Vec<AnimData>>
}

#[derive(Resource)]
pub struct TextureResources {
    pub interior_atlas: Handle<TextureAtlas>,
}

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

#[derive(Component)]
pub struct AnimationData {
    pub current_animation: usize,
}

impl AnimationData {
    pub fn is_current<T: Into<usize>>(&self, animation: T) -> bool {
        self.current_animation == animation.into()
    }

    pub fn set_current<T: Into<usize>>(&mut self, current: T) {
        let animation = current.into();
        self.current_animation = animation;
    }
}

fn update_animation_timer(
    atlas_anim_data: Res<AtlasAnimationData>,
    mut query: Query<(
        Entity,
        &AnimationData,
        &mut TextureAtlasSprite,
        &Handle<TextureAtlas>
    ), Changed<AnimationData>>,
    mut commands: Commands,
) {
    for (entity, data, mut sprite, handle) in &mut query {
        let atlas_data = &atlas_anim_data.data[handle];
        let anim_data = &atlas_data[data.current_animation];
        commands.entity(entity).insert(
            AnimationTimer(Timer::from_seconds(anim_data.delay, TimerMode::Repeating))
        );
        sprite.index = anim_data.index;
    }
}

fn animate_sprite(
    time: Res<Time>,
    atlas_animation_data: Res<AtlasAnimationData>,
    mut query: Query<(
        &mut AnimationTimer,
        &AnimationData,
        &mut TextureAtlasSprite,
        &Handle<TextureAtlas>,
    )>,
) {
    for (mut timer, data, mut sprite, handle) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            let atlas_data = &atlas_animation_data.data[handle];
            let frames = &atlas_data[data.current_animation];
            sprite.index += 1;
            if sprite.index >= frames.index + frames.frames {
                sprite.index = frames.index;
            }
        }
    }
}
