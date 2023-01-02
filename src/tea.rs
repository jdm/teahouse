use bevy::prelude::*;
use bevy::utils::Instant;
use crate::animation::TextureResources;
use crate::entity::Item;
use crate::geom::{TILE_SIZE, MapSize, MapPos, HasSize, map_to_screen};
use crate::interaction::{Interactable, PlayerInteracted};
use crate::map::Map;
use crate::message_line::{DEFAULT_EXPIRY, StatusEvent};
use crate::movable::Movable;
use crate::player::Player;
use rand_derive2::RandGen;
use std::collections::HashMap;
use std::time::Duration;

pub struct TeaPlugin;

impl Plugin for TeaPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<SpawnTeapotEvent>()
            .add_system(spawn_teapot)
            .add_system(interact_with_stash)
            .add_system(interact_with_cupboards)
            .add_system(interact_with_kettles);
    }
}

#[derive(Component)]
pub struct Kettle;

#[derive(Component)]
pub struct Cupboard {
    pub teapots: u32,
}

#[derive(Component)]
pub struct TeaStash {
    pub ingredient: Ingredient,
    pub amount: u32,
}

#[derive(Component, Default, Clone)]
pub struct TeaPot {
    pub ingredients: HashMap<Ingredient, u32>,
    pub steeped_at: Option<Instant>,
    pub steeped_for: Option<Duration>,
    pub water: u32,
}

#[derive(Hash, RandGen, Copy, Clone, PartialEq, Eq, Debug)]
pub enum Ingredient {
    BlackTea,
    OolongTea,
    Chai,
    CitrusPeel,
    MintLeaf,
    Sugar,
    Honey,
    Milk,
    Lemon,
    Rooibos,
    GreenTea,
    Matcha,
    Ceylon,
    Cream,
    BrownSugar,
}

fn interact_with_stash(
    mut q: Query<&mut Player>,
    mut player_interacted_events: EventReader<PlayerInteracted>,
    mut stash: Query<&mut TeaStash>,
    mut status_events: EventWriter<StatusEvent>,
) {
    for event in player_interacted_events.iter() {
        let mut player = q.get_mut(event.player_entity).unwrap();
        let mut stash = match stash.get_mut(event.interacted_entity) {
            Ok(result) => result,
            Err(_) => continue,
        };

        if stash.amount > 0 {
            stash.amount -= 1;
            let amount = player.carrying.entry(stash.ingredient).or_insert(0);
            *amount += 1;
            status_events.send(StatusEvent::timed_message(
                event.player_entity,
                format!("You take a little {:?} ({} remaining)", stash.ingredient, stash.amount),
                DEFAULT_EXPIRY,
            ));
        }
    }
}

fn interact_with_cupboards(
    mut player_interacted_events: EventReader<PlayerInteracted>,
    mut cupboards: Query<&mut Cupboard>,
    mut status_events: EventWriter<StatusEvent>,
    teapot: Query<&TeaPot, With<Player>>,
    mut commands: Commands,
) {
    for event in player_interacted_events.iter() {
        let mut cupboard = match cupboards.get_mut(event.interacted_entity) {
            Ok(result) => result,
            Err(_) => continue,
        };
        let message = if cupboard.teapots > 0 {
            if teapot.is_empty() {
                cupboard.teapots -= 1;
                commands.entity(event.player_entity).insert(TeaPot::default());
                format!("You take a teapot ({} left).", cupboard.teapots)
            } else {
                "You're already carrying a teapot.".to_string()
            }
        } else {
            "No teapots remaining.".to_string()
        };
        status_events.send(StatusEvent::timed_message(
            event.player_entity,
            message.to_string(),
            DEFAULT_EXPIRY,
        ));
    }
}

fn interact_with_kettles(
    mut player_interacted_events: EventReader<PlayerInteracted>,
    mut player: Query<(&mut Player, Option<&mut TeaPot>)>,
    kettles: Query<&Kettle>,
    mut status_events: EventWriter<StatusEvent>,
    time: Res<Time>,
) {
    for event in player_interacted_events.iter() {
        let (mut player, teapot) = player.get_mut(event.player_entity).unwrap();
        let _kettle = match kettles.get(event.interacted_entity) {
            Ok(result) => result,
            Err(_) => continue,
        };
        let mut teapot = match teapot {
            Some(teapot) => teapot,
            None => {
                status_events.send(StatusEvent::timed_message(
                    event.player_entity,
                    "You need a teapot to use the kettle.".to_owned(),
                    DEFAULT_EXPIRY,
                ));
                continue;
            }
        };

        let message = if !player.carrying.is_empty() {
            let ingredients = player
                .carrying
                .keys()
                .map(|k| format!("{:?}", k))
                .collect::<Vec<_>>();

            teapot.water = 100;
            teapot.ingredients = std::mem::take(&mut player.carrying);
            teapot.steeped_at = Some(time.last_update().unwrap());

            let ingredients = ingredients.join(" and the ");
            format!("You add the {} to the teapot and fill it with boiling water.", ingredients)
        } else {
            "You need ingredients to steep before adding the water.".to_owned()
        };

        status_events.send(StatusEvent::timed_message(
            event.player_entity,
            message,
            DEFAULT_EXPIRY,
        ));
    }
}

pub struct SpawnTeapotEvent(pub MapPos);

fn spawn_teapot(
    mut events: EventReader<SpawnTeapotEvent>,
    textures: Res<TextureResources>,
    mut commands: Commands,
    map: Res<Map>,
) {
    for event in events.iter() {
        let size = Vec2::new(TILE_SIZE, TILE_SIZE);
        let map_size = MapSize { width: 1, height: 1 };
        let rect = map_to_screen(&event.0, &map_size, &map);
        // FIXME: make better Z defaults and share them.
        let pos = Vec3::new(rect.x, rect.y, 0.1);

        let sprite = SpriteBundle {
            sprite: Sprite {
                custom_size: Some(size),
                ..default()
            },
            texture: textures.teapot.clone(),
            transform: Transform::from_translation(pos),
            ..default()
        };

        let movable = Movable {
            size: Vec2::new(rect.w, rect.h),
            ..default()
        };
        let sized = HasSize {
            size: map_size,
        };

        commands.spawn((
            TeaPot::default(),
            Item,
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
}
