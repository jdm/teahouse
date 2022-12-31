use bevy::prelude::*;

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_system(animate_sprite);
    }
}

pub struct AnimData {
    pub index: usize,
    pub frames: usize,
}

#[derive(Resource)]
pub struct TextureResources {
    // FIXME: generalize to more atlases
    pub atlas: Handle<TextureAtlas>,
    pub frame_data: Vec<AnimData>,
}

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

#[derive(Component)]
pub struct AnimationData {
    pub current_animation: usize,
}

impl AnimationData {
    pub fn set_current<T: Into<usize>>(&mut self, current: T) {
        let animation = current.into();
        self.current_animation = animation;
    }
}

fn animate_sprite(
    time: Res<Time>,
    texture_resources: Res<TextureResources>,
    mut query: Query<(
        &mut AnimationTimer,
        &AnimationData,
        &mut TextureAtlasSprite,
    )>,
) {
    for (mut timer, data, mut sprite) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            let frames = &texture_resources.frame_data[data.current_animation];
            sprite.index = frames.index + (sprite.index + 1) % frames.frames;
        }
    }
}
