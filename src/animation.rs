use bevy::prelude::*;

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_system(update_animation_timer)
            .add_system(animate_sprite);
    }
}

pub struct AnimData {
    pub index: usize,
    pub frames: usize,
    pub delay: f32,
}

#[derive(Resource)]
pub struct TextureResources {
    // FIXME: generalize to more atlases
    pub atlas: Handle<TextureAtlas>,
    pub interior_atlas: Handle<TextureAtlas>,
    pub frame_data: Vec<AnimData>,
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
    texture_resources: Res<TextureResources>,
    mut query: Query<(Entity, &AnimationData, &mut TextureAtlasSprite), Changed<AnimationData>>,
    mut commands: Commands,
) {
    for (entity, data, mut sprite) in &mut query {
        let anim_data = &texture_resources.frame_data[data.current_animation];
        commands.entity(entity).insert(
            AnimationTimer(Timer::from_seconds(anim_data.delay, TimerMode::Repeating))
        );
        sprite.index = anim_data.index;
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
            sprite.index += 1;
            if sprite.index >= frames.index + frames.frames {
                sprite.index = frames.index;
            }
        }
    }
}
