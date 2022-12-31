use bevy::prelude::*;
use bevy::utils::Instant;
use crate::entity::{Item, Player};
use crate::interaction::Interactable;
use crate::message_line::{DEFAULT_EXPIRY, StatusEvent};
use rand_derive2::RandGen;
use std::collections::HashMap;
use std::time::Duration;

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
}

pub fn interact(
    mut player: Query<Entity, With<Player>>,
    teapots: Query<(Entity, &TeaPot, &Interactable), With<Item>>,
    mut commands: Commands,
    mut status_events: EventWriter<StatusEvent>,
    keys: Res<Input<KeyCode>>,
) {
    if keys.just_released(KeyCode::X) {
        let player_entity = player.single_mut();
        for (teapot_entity, _teapot, interactable) in &teapots {
            if !interactable.colliding {
                continue;
            }

            commands.entity(teapot_entity).despawn();
            commands.entity(player_entity).insert(TeaPot::default());
            status_events.send(StatusEvent::timed_message(
                teapot_entity,
                "You collect the used teapot.".to_owned(),
                DEFAULT_EXPIRY,
            ));
        }
    }
}
