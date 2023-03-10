use bevy::prelude::*;
use crate::GameState;
use crate::animation::TextureResources;
use crate::bookshelf::spawn_bookshelf;
use crate::cat::{CatBed, SpawnCatEvent};
use crate::geom::{HasSize, MapSize, TILE_SIZE, map_to_screen, MapPos};
use crate::map::Map;
use crate::movable::Movable;
use crate::menu::{StartingIngredients, spawn_menu};
use crate::player::SpawnPlayerEvent;
use crate::stair::spawn_staircase;
use crate::tea::{SpawnTeapotEvent, spawn_cupboard, spawn_kettle, spawn_teastash, spawn_sink};
use rand_derive2::RandGen;
use std::default::Default;
use tiled::{LayerType, PropertyValue, ObjectShape};

#[derive(Component)]
pub struct Paused;

#[derive(Component)]
pub struct Item;

#[derive(Copy, Clone, Debug)]
pub enum FacingDirection {
    Up,
    Down,
    Left,
    Right,
}

impl FacingDirection {
    pub fn offset(&self) -> (isize, isize) {
        match self {
            FacingDirection::Up => (0, -1),
            FacingDirection::Down => (0, 1),
            FacingDirection::Left => (-1, 0),
            FacingDirection::Right => (1, 0),
        }
    }

    pub fn to_translation(&self) -> Vec2 {
        let offset = self.offset();
        Vec2::new(
            offset.0 as f32 * TILE_SIZE,
            -offset.1 as f32 * TILE_SIZE,
        )
    }

    pub fn adjust_pos(&self, pos: &MapPos) -> MapPos {
        let offset = self.offset();
        MapPos {
            x: (pos.x as isize + offset.0) as usize,
            y: (pos.y as isize + offset.1) as usize,
        }
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct Facing(pub FacingDirection);

#[derive(Component, Default, Clone)]
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

#[derive(RandGen, PartialEq, Copy, Clone)]
pub enum RelationshipStatus {
    Angry,
    Neutral,
    Friendly,
    VeryFriendly,
    Crushing,
}

const AFFECTION_LEVELS: &[(RelationshipStatus, f32, f32)] = &[
    (RelationshipStatus::Angry, f32::NEG_INFINITY, 0.),
    (RelationshipStatus::Neutral, 0., 1.5),
    (RelationshipStatus::Friendly, 1.5, 5.),
    (RelationshipStatus::VeryFriendly, 5., 7.5),
    (RelationshipStatus::Crushing, 7.5, f32::INFINITY), 
];

impl From<RelationshipStatus> for Affection {
    fn from(desired: RelationshipStatus) -> Self {
        let mut value = 0.;
        for (status, lower, upper) in AFFECTION_LEVELS {
            if desired == *status {
                if !upper.is_infinite() {
                    value = *upper;
                    break;
                }
                value = *lower;
                break;
            }
        }
        Affection {
            affection: value,
        }
    }
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
        for (status, lower, upper) in AFFECTION_LEVELS {
            if (lower.is_infinite() || self.affection >= *lower) &&
                (upper.is_infinite() || self.affection < *upper) {
                return *status;
            }
        }
        unimplemented!()
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

const STARTING_INGREDIENT_AMOUNT: u32 = 50;

pub fn setup(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let map: Handle<crate::map::TiledMap> = asset_server.load("teahouse.tmx");
    commands.insert_resource(Map {
        width: 0,
        height: 0,
        handle: map,
    });
}

pub fn setup2(
    mut commands: Commands,
    mut map2: ResMut<Map>,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut teapot_spawner: EventWriter<SpawnTeapotEvent>,
    mut player_spawner: EventWriter<SpawnPlayerEvent>,
    mut cat_spawner: EventWriter<SpawnCatEvent>,
    ingredients: Res<StartingIngredients>,
    assets: Res<Assets<crate::map::TiledMap>>,
) {
    let texture_handle2 = asset_server.load("interiors.png");
    let texture_atlas2 =
        TextureAtlas::from_grid(texture_handle2, Vec2::new(TILE_SIZE, TILE_SIZE), 48, 16, None, None);
    let texture_atlas_handle2 = texture_atlases.add(texture_atlas2);

    let textures = TextureResources {
        interior_atlas: texture_atlas_handle2,
    };

    let mut stashes_spawned = 0;

    let map = assets.get(&map2.handle).unwrap();
    let map = &map.map;
    map2.width = map.width as usize;
    map2.height = map.height as usize;

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
                    let kind = match object.properties.get("kind") {
                        Some(PropertyValue::StringValue(kind)) => kind,
                        _ => continue,
                    };
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
                            commands.spawn((Door, sized, transform));
                        }
                        "catbed" => {
                            commands.spawn((CatBed, sized, transform));
                        }
                        "kettle" => {
                            spawn_kettle(&mut commands, movable, sized, transform);
                        }
                        "teastash" => {
                            spawn_teastash(
                                &mut commands,
                                movable,
                                sized,
                                transform,
                                ingredients.ingredients[stashes_spawned],
                                STARTING_INGREDIENT_AMOUNT,
                            );
                            stashes_spawned += 1;
                        }
                        "sink" => {
                            spawn_sink(&mut commands, movable, sized, transform);
                        }
                        "player" => {
                            player_spawner.send(SpawnPlayerEvent(pos));
                        }
                        "teapot" => {
                            teapot_spawner.send(SpawnTeapotEvent::at(pos));
                        }
                        "cat" => {
                            cat_spawner.send(SpawnCatEvent(pos));
                        }
                        "chair" => {
                            commands.spawn((Chair, sized, transform));
                        }
                        "cupboard" => {
                            spawn_cupboard(
                                &mut commands,
                                movable,
                                sized,
                                transform,
                            );
                        }
                        "menu" => {
                            spawn_menu(&mut commands, movable, sized, transform);
                        }
                        "bookshelves" => {
                            spawn_bookshelf(&mut commands, movable, sized, transform);
                        }
                        "stairs" => {
                            let id = match object.properties.get("id") {
                                Some(PropertyValue::StringValue(id)) => id.to_owned(),
                                p => {
                                    warn!("Unsupported id property value: {:?}", p);
                                    continue;
                                }
                            };

                            let destination = match object.properties.get("destination") {
                                Some(PropertyValue::StringValue(d)) => d.to_owned(),
                                p => {
                                    warn!("Unsupported destination property value: {:?}", p);
                                    continue;
                                }
                            };

                            spawn_staircase(&mut commands, movable, sized, transform, id, destination);
                        }
                        s => warn!("Ignoring unknown object kind: {}", s),
                    }
                }
            }

            _ => warn!("Ignoring unknown layer {}", layer.name)
        }
        z += 0.1;
    }

    commands.insert_resource(textures);
}

pub fn start_game(
    mut game_state: ResMut<State<GameState>>,
) {
    game_state.set(GameState::InGame).unwrap();
}
