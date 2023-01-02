use bevy::prelude::*;
use crate::animation::{AnimationData, AnimData, TextureResources};
use crate::cat::*;
use crate::customer::Customer;
use crate::geom::{HasSize, MapSize, TILE_SIZE, ScreenRect, map_to_screen, MapPos};
use crate::interaction::Interactable;
use crate::map::Map;
use crate::movable::Movable;
use crate::message_line::{StatusMessage, StatusMessageBundle};
use crate::player::Player;
use crate::tea::{Ingredient, TeaStash, TeaPot, Kettle, Cupboard};
use rand::Rng;
use std::default::Default;
use tiled::{Loader, LayerType, PropertyValue, ObjectShape};

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

#[allow(dead_code)]
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

#[allow(dead_code)]
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
                    bundle.transform.scale = Vec3::new(1.0, 1.0, 1.0);
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
            commands.spawn((Prop, movable, sized, sprite));
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
    mut commands: Commands,
    map2: Res<Map>,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let texture_handle = asset_server.load("cat.png");
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, Vec2::new(TILE_SIZE, TILE_SIZE), 4, 5, None, None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    let texture_handle2 = asset_server.load("interiors.png");
    let texture_atlas2 =
        TextureAtlas::from_grid(texture_handle2, Vec2::new(TILE_SIZE, TILE_SIZE), 48, 16, None, None);
    let texture_atlas_handle2 = texture_atlases.add(texture_atlas2);

    let textures = TextureResources {
        atlas: texture_atlas_handle,
        interior_atlas: texture_atlas_handle2,
        frame_data: vec![
            AnimData { index: 0, frames: 4, delay: 0.1, },
            AnimData { index: 4, frames: 4, delay: 0.1, },
            AnimData { index: 8, frames: 4, delay: 0.1, },
            AnimData { index: 12, frames: 4, delay: 0.1, },
            AnimData { index: 16, frames: 1, delay: 0.1, },
            AnimData { index: 17, frames: 2, delay: 0.8, },
        ],
    };

    let mut loader = Loader::new();
    let map = loader.load_tmx_map("assets/teahouse.tmx").unwrap();
    let mut z = 0.;
    for layer in map.layers() {
        let properties = &layer.properties;
        let solid = properties.get("solid").map_or(false, |value| *value == PropertyValue::BoolValue(true));
        println!("{:?}", layer.name);
        match layer.layer_type() {
            LayerType::TileLayer(layer) => {
                for y in 0..map.height {
                    for x in 0..map.width {
                        let tile = match layer.get_tile(x as i32, y as i32) {
                            Some(tile) => tile,
                            None => continue,
                        };
                        let pos = MapPos { x: x as usize, y: y as usize };
                        let size = MapSize { width: 1, height: 1 };
                        let rect = map_to_screen(&pos, &size, &map2);
                        let pos = Vec2::new(rect.x, rect.y);
                        let sprite = SpriteSheetBundle {
                            texture_atlas: textures.interior_atlas.clone(),
                            sprite: TextureAtlasSprite {
                                index: tile.id() as usize,
                                ..default()
                            },
                            transform: Transform::from_translation(pos.extend(z)),
                            ..default()
                        };
                        let sized = HasSize { size };
                        if solid {
                            let size = Vec2::new(rect.w, rect.h);
                            let movable = Movable {
                                size: size,
                                ..default()
                            };
                            commands.spawn((Prop, movable, sized, sprite));
                        } else {
                            commands.spawn((Prop, sized, sprite));
                        }
                    }
                }
            }

            LayerType::ObjectLayer(layer) => {
                for object in layer.objects() {
                    println!("{:?}", *object);
                    let kind = match object.properties.get("kind") {
                        Some(PropertyValue::StringValue(kind)) => kind,
                        _ => continue,
                    };
                    println!("{:?}", kind);
                    let (width, height) = match object.shape {
                        ObjectShape::Rect { width, height } => (width, height),
                        _ => continue,
                    };
                    let size = MapSize {
                        width: (width / TILE_SIZE) as usize,
                        height: (height / TILE_SIZE) as usize,
                    };
                    let movable = Movable {
                        size: Vec2::new(width, height),
                        ..default()
                    };
                    let sized = HasSize { size };
                    let pos = MapPos { x: (object.x / TILE_SIZE) as usize, y: (object.y / TILE_SIZE) as usize };
                    let rect = map_to_screen(&pos, &size, &map2);
                    let transform = Transform {
                        translation: Vec3::new(rect.x, rect.y, 0.),
                        rotation: Quat::default(),
                        scale: Vec3::splat(1.),
                    };

                    match kind.as_str() {
                        "door" => {
                            commands.spawn((Door, movable, sized, transform));
                        }
                        "catbed" => {
                            commands.spawn((CatBed, sized, transform));
                        }
                        "kettle" => {
                            commands.spawn((
                                Kettle,
                                Interactable {
                                    highlight: Color::rgb(1., 1., 1.),
                                    message: "Press X to fill the pot".to_string(),
                                    ..default()
                                },
                                movable,
                                sized,
                                transform,
                            ));
                        }
                        "teastash" => {
                            let ingredient = Ingredient::generate_random();
                            let mut rng = rand::thread_rng();
                            let amount = rng.gen_range(1..10);
                            commands.spawn((
                                TeaStash { ingredient, amount },
                                Interactable {
                                    highlight: Color::rgb(1., 1., 1.),
                                    message: format!("Press X to pick up {:?}", ingredient),
                                    ..default()
                                },
                                movable,
                                sized,
                                transform,
                            ));
                        }
                        "player" => {
                            spawn_sprite(EntityType::Player, rect, &mut commands);
                        }
                        "teapot" => {
                            spawn_sprite(EntityType::TeaPot, rect, &mut commands);
                        }
                        "cat" => {
                            spawn_sprite_inner(EntityType::Cat, rect, &mut commands, Some(&textures));
                        }
                        "chair" => {
                            commands.spawn((Chair, sized, transform));
                        }
                        "cupboard" => {
                            let mut rng = rand::thread_rng();
                            commands.spawn((
                                Cupboard { teapots: rng.gen_range(4..10) },
                                Interactable {
                                    highlight: Color::rgb(1., 1., 1.),
                                    message: "Press X to pick up teapot".to_string(),
                                    ..default()
                                },
                                movable,
                                sized,
                                transform,
                            ));
                        }
                        _ => {}
                    }
                }
            }

            _ => {}
        }
        z += 0.1;
    }

    commands.spawn(
        StatusMessageBundle {
            message: StatusMessage::default(),
            text: TextBundle::from_section(
                "",
                TextStyle {
                    font: asset_server.load("Lato-Medium.ttf"),
                    font_size: 25.0,
                    color: Color::WHITE,
                },
            )
                .with_text_alignment(TextAlignment::TOP_CENTER)
                .with_style(Style {
                    position_type: PositionType::Absolute,
                    position: UiRect {
                        bottom: Val::Px(5.0),
                        right: Val::Px(15.0),
                        ..default()
                    },
                    ..default()
                }),
        }
    );

    commands.insert_resource(textures);
}
