use bevy::prelude::*;
use crate::cat::*;
use crate::customer::Customer;
use crate::geom::*;
use crate::interaction::*;
use crate::map::Map;
use crate::movable::*;
use crate::message_line::{StatusMessage, StatusMessageBundle};
use crate::tea::{Ingredient, TeaStash};
use rand::Rng;
use std::collections::HashMap;
use std::default::Default;

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

#[derive(Component)]
pub struct Prop;

#[derive(Component)]
pub struct Kettle;

#[derive(Component)]
pub struct Stove;

#[derive(Component)]
pub struct Door;

#[derive(Component)]
pub struct CatBed;

#[derive(Component)]
pub struct Chair;

#[derive(Component)]
pub struct Cupboard {
    pub teapots: u32,
}

#[derive(Component, Default)]
pub struct Player {
    pub carrying: HashMap<Ingredient, u32>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum EntityType {
    Customer(Vec<String>),
    Player,
    Prop,
    Chair,
    Door,
    Stove,
    TeaStash(Ingredient, u32),
    Cupboard(u32),
    CatBed,
    Cat,
    Kettle,
}

pub const CAT_SPEED: f32 = 25.0;
pub const CUSTOMER_SPEED: f32 = 40.0;
pub const SPEED: f32 = 150.0;

pub fn spawn_sprite(entity: EntityType, rect: ScreenRect, commands: &mut Commands) {
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
        EntityType::Customer(..) => Color::rgb(0.0, 0.25, 0.0),
        EntityType::Prop => Color::rgb(0.25, 0.15, 0.0),
        EntityType::Chair => Color::rgb(0.15, 0.05, 0.0),
        EntityType::Door => Color::rgb(0.6, 0.2, 0.2),
        EntityType::Stove => Color::rgb(0.8, 0.8, 0.8),
        EntityType::TeaStash(..) => Color::rgb(0.3, 0.3, 0.3),
        EntityType::Cupboard(..) => Color::rgb(0.5, 0.35, 0.0),
        EntityType::CatBed => Color::rgb(0., 0., 0.25),
        EntityType::Cat => Color::BLACK,
        EntityType::Kettle => Color::LIME_GREEN,
    };
    let sprite = SpriteBundle {
        sprite: Sprite {
            color,
            custom_size: Some(size),
            ..default()
        },
        transform: Transform::from_translation(pos.extend(0.)),
        ..default()
    };
    let movable = Movable { speed: Vec2::ZERO, size: size, entity_speed: speed };
    let sized = HasSize {
        size: MapSize {
            width: (rect.w / TILE_SIZE) as usize,
            height: (rect.h / TILE_SIZE) as usize,
        }
    };
    match entity {
        EntityType::Player => {
            commands.spawn((Player::default(), movable, sized, sprite))
                .with_children(|parent| {
                    parent.spawn(Camera2dBundle::default());
                });
        }
        EntityType::Customer(conversation) => {
            commands.spawn((
                Customer {
                    conversation,
                    ..default()
                },
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
        EntityType::Cat => {
            commands.spawn((
                Cat::default(),
                Affection::default(),
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
        EntityType::Chair => {
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
    };
}

pub fn setup(
    mut commands: Commands,
    mut map: ResMut<Map>,
    asset_server: Res<AssetServer>,
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
        spawn_sprite(
            entity_type,
            rect,
            &mut commands,
        );
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
}
