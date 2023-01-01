use bevy::prelude::*;
use crate::animation::{AnimationData, /*AnimData,*/ TextureResources};
use crate::cat::*;
use crate::customer::Customer;
use crate::geom::{HasSize, MapSize, TILE_SIZE, ScreenRect, map_to_screen};
use crate::interaction::Interactable;
use crate::map::Map;
use crate::movable::Movable;
use crate::player::Player;
use crate::tea::{Ingredient, TeaStash, TeaPot, Kettle, Cupboard};
use rand::Rng;
use std::default::Default;

#[derive(Component)]
pub struct Paused;

#[derive(Component)]
pub struct Item(pub EntityType);

pub enum FacingDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Component)]
pub struct Facing(pub FacingDirection);

#[derive(Component, Default)]
pub struct Affection {
    affection: f32,
}

#[allow(dead_code)]
pub enum Reaction {
    Positive,
    MajorPositive,
    Negative,
    MajorNegative,
}

#[allow(dead_code)]
pub enum RelationshipStatus {
    Angry,
    Neutral,
    Friendly,
    VeryFriendly,
    Crushing,
}

impl Affection {
    pub fn react(&mut self, reaction: Reaction) {
        self.affection += match reaction {
            Reaction::Positive => 0.25,
            Reaction::MajorPositive => 1.,
            Reaction::Negative => -0.25,
            Reaction::MajorNegative => -1.
        };
    }

    pub fn status(&self) -> RelationshipStatus {
        if self.affection < 0. {
            RelationshipStatus::Angry
        } else if self.affection < 1.5 {
            RelationshipStatus::Neutral
        } else if self.affection < 5. {
            RelationshipStatus::Friendly
        } else if self.affection < 7.5 {
            RelationshipStatus::VeryFriendly
        } else {
            RelationshipStatus::Crushing
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TileDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Component)]
pub struct Prop;

#[derive(Component)]
pub struct Stove;

#[derive(Component)]
pub struct Door;

#[derive(Component)]
pub struct Chair;

#[derive(Clone, PartialEq, Debug)]
pub enum EntityType {
    Customer(Color),
    Player,
    Prop,
    Chair(TileDirection),
    Door,
    Stove,
    TeaStash(Ingredient, u32),
    Cupboard(u32),
    CatBed,
    Cat,
    Kettle,
    TeaPot,
}

pub const CAT_SPEED: f32 = 25.0;
pub const CUSTOMER_SPEED: f32 = 40.0;
pub const SPEED: f32 = 150.0;

pub fn spawn_sprite(
    entity: EntityType,
    rect: ScreenRect,
    commands: &mut Commands,
) {
    spawn_sprite_inner(entity, rect, commands, None)
}

fn spawn_sprite_inner(
    entity: EntityType,
    rect: ScreenRect,
    commands: &mut Commands,
    textures: Option<&TextureResources>,
) {
    let pos = Vec2::new(rect.x, rect.y);
    let size = Vec2::new(rect.w, rect.h);
    let speed = match entity {
        EntityType::Player => SPEED,
        EntityType::Customer(..) => CUSTOMER_SPEED,
        EntityType::Cat => CAT_SPEED,
        _ => 0.,
    };
    let color = match entity {
        EntityType::Player => Color::rgb(0.25, 0.25, 0.75),
        EntityType::Customer(color) => color,
        EntityType::Prop => Color::rgb(0.25, 0.15, 0.0),
        EntityType::Chair(..) => Color::rgb(0.15, 0.05, 0.0),
        EntityType::Door => Color::rgb(0.6, 0.2, 0.2),
        EntityType::Stove => Color::rgb(0.8, 0.8, 0.8),
        EntityType::TeaStash(..) => Color::rgb(0.3, 0.3, 0.3),
        EntityType::Cupboard(..) => Color::rgb(0.5, 0.35, 0.0),
        EntityType::CatBed => Color::rgb(0., 0., 0.25),
        EntityType::Cat => Color::BLACK,
        EntityType::Kettle => Color::LIME_GREEN,
        EntityType::TeaPot => Color::GRAY,
    };
    let z = match entity {
        EntityType::Chair(..) | EntityType::CatBed => 0.,
        _ => 0.1,
    };
    let sprite = SpriteBundle {
        sprite: Sprite {
            color,
            custom_size: Some(size),
            ..default()
        },
        transform: Transform::from_translation(pos.extend(z)),
        ..default()
    };
    let movable = Movable {
        speed: Vec2::ZERO,
        size: size,
        entity_speed: speed,
        subtile_max: None,
    };
    let sized = HasSize {
        size: MapSize {
            width: (rect.w / TILE_SIZE) as usize,
            height: (rect.h / TILE_SIZE) as usize,
        }
    };
    match entity {
        EntityType::Player => {
            commands.spawn((
                Player::default(),
                Facing(FacingDirection::Down),
                movable,
                sized,
                sprite,
            ))
                .with_children(|parent| {
                    let mut bundle = Camera2dBundle::default();
                    bundle.transform.scale = Vec3::new(0.75, 0.75, 1.0);
                    parent.spawn(bundle);
                });
        }
        EntityType::Customer(..) => {
            commands.spawn((
                Customer::default(),
                Affection::default(),
                Interactable {
                    highlight: Color::rgb(1., 1., 1.),
                    message: "Press X to talk".to_string(),
                    ..default()
                },
                movable,
                sized,
                sprite,
            ));
        }
        EntityType::TeaPot => {
            commands.spawn((
                TeaPot::default(),
                Item(EntityType::TeaPot),
                Interactable {
                    highlight: Color::rgb(1., 1., 1.),
                    message: "Press X to collect".to_string(),
                    ..default()
                },
                movable,
                sized,
                sprite,
            ));
        }
        EntityType::Cat => {
            let sprite = SpriteSheetBundle {
                texture_atlas: textures.unwrap().atlas.clone(),
                transform: Transform::from_translation(pos.extend(z)),
                ..default()
            };

            commands.spawn((
                Cat::default(),
                AnimationData { current_animation: CatAnimationState::Sleep.into() },
                Affection::default(),
                Facing(FacingDirection::Down),
                crate::cat::State::default(),
                Interactable {
                    highlight: Color::rgb(1., 1., 1.),
                    message: "Press X to pet the cat".to_string(),
                    ..default()
                },
                movable,
                sized,
                sprite,
            ));
        }
        EntityType::Chair(dir) => {
            let y_index = match dir {
                TileDirection::Right => 0,
                TileDirection::Down => 1,
                TileDirection::Left => 2,
                TileDirection::Up => 3,
            };
            let sprite = SpriteSheetBundle {
                texture_atlas: textures.unwrap().interior_atlas.clone(),
                sprite: TextureAtlasSprite {
                    index: ((6 + y_index) * 48 + 15),
                    ..default()
                },
                transform: Transform::from_translation(pos.extend(z)),
                ..default()
            };

            commands.spawn((Chair, sized, sprite));
        }
        EntityType::Prop => {
            commands.spawn((Prop, movable, sized/*, sprite*/));
        }
        EntityType::Door => {
            commands.spawn((Door, movable, sized, sprite));
        }
        EntityType::Kettle => {
            commands.spawn((
                Kettle,
                Interactable {
                    highlight: Color::rgb(1., 1., 1.),
                    message: "Press X to fill the pot".to_string(),
                    ..default()
                },
                movable,
                sized,
                sprite,
            ));
        }
        EntityType::Stove => {
            commands.spawn((
                Stove,
                Interactable {
                    highlight: Color::rgb(1., 1., 1.),
                    message: "Press X to toggle burner".to_string(),
                    ..default()
                },
                movable,
                sized,
                sprite,
            ));
        }
        EntityType::TeaStash(ingredient, amount) => {
            commands.spawn((
                TeaStash { ingredient, amount },
                Interactable {
                    highlight: Color::rgb(1., 1., 1.),
                    message: format!("Press X to pick up {:?}", ingredient),
                    ..default()
                },
                movable,
                sized,
                sprite,
            ));
        }
        EntityType::Cupboard(pots) => {
            commands.spawn((
                Cupboard { teapots: pots },
                Interactable {
                    highlight: Color::rgb(1., 1., 1.),
                    message: "Press X to pick up teapot".to_string(),
                    ..default()
                },
                movable,
                sized,
                sprite,
            ));
        }
        EntityType::CatBed => {
            commands.spawn((
                CatBed,
                sized,
                sprite,
            ));
        }
    }
}

pub fn setup(
    mut commands: &mut Commands,
    //mut map: ResMut<Map>,
    map: &mut Map,
    //texture_resources: Res<TextureResources>,
    texture_resources: &TextureResources,
) {

    for pos in &map.cat_beds {
        let rect = map_to_screen(pos, &MapSize { width: 2, height: 2 }, &map);
        spawn_sprite(
            EntityType::CatBed,
            rect,
            &mut commands,
        )
    }

    for pos in &map.cupboards {
        let rect = map_to_screen(pos, &MapSize { width: 2, height: 1 }, &map);
        let mut rng = rand::thread_rng();
        spawn_sprite(
            EntityType::Cupboard(rng.gen_range(4..10)),
            rect,
            &mut commands,
        )
    }

    for (size, pos) in &map.props {
        let rect = map_to_screen(pos, size, &map);
        spawn_sprite(
            EntityType::Prop,
            rect,
            &mut commands,
        )
    }

    for (entity_type, pos) in std::mem::take(&mut map.entities) {
        let size = MapSize { width: 1, height: 1 };
        let rect = map_to_screen(&pos, &size, &map);
        spawn_sprite_inner(
            entity_type,
            rect,
            &mut commands,
            Some(&texture_resources),
        );
    }
}
