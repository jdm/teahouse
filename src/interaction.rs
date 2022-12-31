use bevy::prelude::*;
use bevy::sprite::collide_aabb::collide;
use crate::GameState;
use crate::cat::{Cat, petting_reaction};
use crate::customer::{Customer, tea_delivery, conversation};
use crate::dialog::show_message_box;
use crate::entity::*;
use crate::geom::HasSize;
use crate::movable::Movable;
use crate::message_line::{DEFAULT_EXPIRY, StatusEvent};
use crate::tea::{TeaPot, TeaStash};

#[derive(Component)]
pub struct Interactable {
    pub highlight: Color,
    pub previous: Option<Color>,
    pub colliding: bool,
    pub message: String,
}

impl Default for Interactable {
    fn default() -> Self {
        Self {
            highlight: Color::BLACK,
            previous: None,
            colliding: false,
            message: String::new(),
        }
    }
}

pub fn highlight_interactable(
    player: Query<(&Transform, &Movable), (With<Player>, Changed<Transform>)>,
    mut interactable: Query<(Entity, &mut Interactable, &Transform, Option<&mut Sprite>, &HasSize), Changed<Transform>>,
    mut status_events: EventWriter<StatusEvent>,
) {
    if player.is_empty() {
        return;
    }

    let (player_transform, player_movable) = player.single();

    for (entity, mut interactable, transform, mut sprite, size) in interactable.iter_mut() {
        let screen_size = size.screen_size();
        let collision = collide(
            transform.translation,
            Vec2::new(screen_size.0, screen_size.1),
            player_transform.translation,
            player_movable.size * 1.3,
        );
        if collision.is_some() {
            if interactable.previous.is_none() {
                if let Some(ref mut sprite) = sprite {
                    interactable.previous = Some(sprite.color);
                    sprite.color = interactable.highlight;
                } else {
                    interactable.previous = Some(Color::WHITE);
                }

                status_events.send(StatusEvent::timed_message(
                    entity,
                    interactable.message.clone(),
                    DEFAULT_EXPIRY,
                ));
            }
            interactable.colliding = true;
        } else if collision.is_none() && interactable.previous.is_some() {
            let previous = interactable.previous.take();
            if let Some(ref mut sprite) = sprite {
                sprite.color = previous.unwrap();
            }
            interactable.colliding = false;

            status_events.send(StatusEvent::clear(entity));
        }
    }
}

pub struct PlayerInteracted {
    player_entity: Entity,
    interacted_entity: Entity,
}

pub fn interact_with_stash(
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

pub fn interact_with_cupboards(
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

pub fn interact_with_customers(
    mut player_interacted_events: EventReader<PlayerInteracted>,
    mut customers: Query<(Entity, &mut Affection), With<Customer>>,
    teapot: Query<&TeaPot, With<Player>>,
    asset_server: Res<AssetServer>,
    mut game_state: ResMut<State<GameState>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for event in player_interacted_events.iter() {
        let (customer_entity, mut affection) = match customers.get_mut(event.interacted_entity) {
            Ok(result) => result,
            Err(_) => continue,
        };
        if !teapot.is_empty() {
            let teapot = teapot.single();

            if teapot.steeped_at.is_some() {
                commands.entity(event.player_entity).remove::<TeaPot>();
                let mut delivered = (*teapot).clone();
                //FIXME: wasm issues
                delivered.steeped_for = Some(time.last_update().unwrap() - delivered.steeped_at.unwrap());
                commands.entity(customer_entity).insert(delivered);

                let (reaction, conversation) = tea_delivery(&teapot);
                affection.react(reaction);
                game_state.set(GameState::Dialog).unwrap();
                show_message_box(customer_entity, &mut commands, conversation, asset_server);
                return;
            }
        }

        game_state.set(GameState::Dialog).unwrap();
        show_message_box(customer_entity, &mut commands, conversation(), asset_server);
        return;
    }
}

pub fn interact_with_cat(
    mut player_interacted_events: EventReader<PlayerInteracted>,
    mut cat: Query<(&Cat, &mut Affection)>,
    mut status_events: EventWriter<StatusEvent>,
) {
    for event in player_interacted_events.iter() {
        let (cat, mut affection) = match cat.get_mut(event.interacted_entity) {
            Ok(result) => result,
            Err(_) => continue,
        };
        let (reaction, message) = petting_reaction(&cat, &affection);
        status_events.send(StatusEvent::timed_message(
            event.player_entity,
            message,
            DEFAULT_EXPIRY,
        ));

        affection.react(reaction);
    }
}

pub fn interact_with_kettles(
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

pub fn keyboard_input(
    keys: Res<Input<KeyCode>>,
    mut q: Query<(Entity, &mut Movable), With<Player>>,
    mut interacted_events: EventWriter<PlayerInteracted>,
    interactables: Query<(Entity, &Interactable)>,
) {
    let (player_entity, mut movable) = q.single_mut();

    if keys.just_pressed(KeyCode::Up) {
        movable.speed.y = SPEED;
    } else if keys.just_pressed(KeyCode::Down) {
        movable.speed.y = -SPEED;
    }
    if keys.any_just_released([KeyCode::Up, KeyCode::Down]) {
        movable.speed.y = 0.0;
    }

    if keys.just_pressed(KeyCode::Left) {
        movable.speed.x = -SPEED;
    } else if keys.just_pressed(KeyCode::Right) {
        movable.speed.x = SPEED;
    }
    if keys.any_just_released([KeyCode::Left, KeyCode::Right]) {
        movable.speed.x = 0.0;
    }

    if keys.just_released(KeyCode::X) {
        for (entity, interactable) in &interactables {
            if !interactable.colliding {
                continue;
            }
            interacted_events.send(PlayerInteracted {
                player_entity,
                interacted_entity: entity,
            });
        }
    }
}
