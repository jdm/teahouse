use bevy::prelude::*;
use bevy::core_pipeline::clear_color::ClearColorConfig;
use crate::animation::{AtlasAnimationData, AnimData, AnimationData};
use crate::entity::{Facing, FacingDirection};
use crate::geom::{TILE_SIZE, HasSize, MapSize, MapPos, map_to_screen};
use crate::map::Map;
use crate::movable::Movable;
use crate::tea::Ingredient;
use std::collections::HashMap;

pub const SPEED: f32 = 150.0;

#[derive(Component)]
pub struct Holding {
    pub entity: Entity,
}

#[derive(Component, Default)]
pub struct Player {
    pub carrying: HashMap<Ingredient, u32>,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_startup_system(init_texture)
            .add_system(adjust_held_item)
            .add_system(spawn_player)
            .add_system(stop_animation)
            .add_event::<SpawnPlayerEvent>();
    }
}

fn stop_animation(
    mut query: Query<(&mut AnimationData, &Facing, &Movable), (With<Player>, Changed<Movable>)>,
) {
    if query.is_empty() {
        return;
    }

    let (mut animation, facing, movable) = query.single_mut();
    if movable.speed != Vec2::ZERO {
        return;
    }
    animation.set_current(standing_conversion(facing.0));
}

fn adjust_held_item(
    holder: Query<(&Holding, &Facing), Or<(Changed<Holding>, Changed<Facing>)>>,
    mut held_transform: Query<&mut Transform>,
) {
    for (holding, facing) in &holder {
        let mut transform = match held_transform.get_mut(holding.entity) {
            Ok(transform) => transform,
            Err(_) => continue,
        };
        transform.translation = facing.to_translation()
            .extend(transform.translation.z);
    }
}

pub struct SpawnPlayerEvent(pub MapPos);

#[derive(Resource)]
struct PlayerTexture(Handle<TextureAtlas>);

fn init_texture(
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut animation_data: ResMut<AtlasAnimationData>,
    mut commands: Commands,
) {
    let texture_handle = asset_server.load("player.png");
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, Vec2::new(TILE_SIZE, TILE_SIZE), 4, 4, None, None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    animation_data.data.insert(
        texture_atlas_handle.clone(),
        vec![
            AnimData { index: 0, frames: 4, delay: 0.1, },
            AnimData { index: 4, frames: 4, delay: 0.1, },
            AnimData { index: 8, frames: 4, delay: 0.1, },
            AnimData { index: 12, frames: 4, delay: 0.1, },
            AnimData { index: 0, frames: 1, delay: 1., },
            AnimData { index: 4, frames: 1, delay: 1., },
            AnimData { index: 8, frames: 1, delay: 1., },
            AnimData { index: 12, frames: 1, delay: 1., },
        ],
    );
    commands.insert_resource(PlayerTexture(texture_atlas_handle));
}

fn spawn_player(
    mut events: EventReader<SpawnPlayerEvent>,
    mut commands: Commands,
    texture: Res<PlayerTexture>,
    map: Res<Map>,
) {
    let size = MapSize {
        width: 1,
        height: 1,
    };
    for event in events.iter() {
        let screen_rect = map_to_screen(&event.0, &size, &map);
        let z = 0.1;
        let screen_size = Vec2::new(screen_rect.w, screen_rect.h);
        let movable = Movable {
            speed: Vec2::ZERO,
            size: screen_size,
            entity_speed: SPEED,
            subtile_max: None,
        };
        let sized = HasSize { size };
        let pos = Vec3::new(screen_rect.x, screen_rect.y, z);
        let transform = Transform::from_translation(pos);

        let sprite = SpriteSheetBundle {
            texture_atlas: texture.0.clone(),
            transform,
            ..default()
        };

        commands.spawn((
            Player::default(),
            Facing(FacingDirection::Down),
            AnimationData {
                current_animation: AnimationState::StandDown.into(),
                facing_conversion,
            },
            movable,
            sized,
            sprite,
        ))
            .with_children(|parent| {
                let mut bundle = Camera2dBundle::default();
                bundle.camera_2d.clear_color = ClearColorConfig::Custom(Color::BLACK);
                bundle.transform.scale = Vec3::new(1.0, 1.0, 1.0);
                parent.spawn(bundle);
            });
    }
}

#[derive(Copy, Clone)]
enum AnimationState {
    WalkDown = 0,
    WalkRight = 1,
    WalkLeft = 2,
    WalkUp = 3,
    StandDown = 4,
    StandRight = 5,
    StandLeft = 6,
    StandUp = 7,
}

impl From<AnimationState> for usize {
    fn from(state: AnimationState) -> usize {
        state as usize
    }
}

fn standing_conversion(facing: FacingDirection) -> AnimationState {
    match facing {
        FacingDirection::Up => AnimationState::StandUp,
        FacingDirection::Down => AnimationState::StandDown,
        FacingDirection::Right => AnimationState::StandRight,
        FacingDirection::Left => AnimationState::StandLeft,
    }
}

fn facing_conversion(facing: FacingDirection) -> usize {
    match facing {
        FacingDirection::Up => AnimationState::WalkUp,
        FacingDirection::Down => AnimationState::WalkDown,
        FacingDirection::Right => AnimationState::WalkRight,
        FacingDirection::Left => AnimationState::WalkLeft,
    }.into()
}
