use bevy::prelude::*;
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
            .add_event::<SpawnPlayerEvent>();
    }
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
struct PlayerTexture(Handle<Image>);

fn init_texture(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let people_handle = asset_server.load("people.png");
    commands.insert_resource(PlayerTexture(people_handle));
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

        let sprite = SpriteBundle {
            sprite: Sprite {
                custom_size: Some(screen_size),
                rect: Some(Rect::new(0., 0., TILE_SIZE, TILE_SIZE)),
                ..default()
            },
            texture: texture.0.clone(),
            transform,
            ..default()
        };

        commands.spawn((
            Player::default(),
            Facing(FacingDirection::Down),
            movable,
            sized,
            sprite,
        ))
            .with_children(|parent| {
                let mut bundle = Camera2dBundle::default();
                bundle.transform.scale = Vec3::new(1.0, 1.0, 1.0);
                parent.spawn(bundle);
            });
    }
}

