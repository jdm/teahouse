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
    player: Query<(&Transform, &Movable), With<Player>>,
    mut interactable: Query<(Entity, &mut Interactable, &Transform, Option<&mut Sprite>, &HasSize)>,
    mut status_events: EventWriter<StatusEvent>,
) {
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
            break;
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

pub fn keyboard_input(
    keys: Res<Input<KeyCode>>,
    mut q: Query<(Entity, &mut Player, &mut Movable)>,
    mut interactables: Query<(Entity, &mut TeaStash, &Interactable)>,
    mut cupboards: Query<(Entity, &mut Cupboard, &Interactable)>,
    mut customers: Query<(Entity, &Customer, &mut Affection, &Interactable), Without<Cat>>,
    mut cat: Query<(Entity, &Cat, &Interactable, &mut Affection)>,
    kettles: Query<(Entity, &Interactable), With<Kettle>>,
    mut commands: Commands,
    mut game_state: ResMut<State<GameState>>,
    mut teapot: Query<&mut TeaPot, With<Player>>,
    asset_server: Res<AssetServer>,
    mut status_events: EventWriter<StatusEvent>,
    time: Res<Time>,
) {
    let (player_entity, mut player, mut movable) = q.single_mut();

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
        for (_entity, mut stash, interactable) in &mut interactables {
            if interactable.colliding {
                stash.amount -= 1;
                let amount = player.carrying.entry(stash.ingredient).or_insert(0);
                *amount += 1;
                status_events.send(StatusEvent::timed_message(
                    player_entity,
                    format!("You take a little {:?} ({} remaining)", stash.ingredient, stash.amount),
                    DEFAULT_EXPIRY,
                ));
                return;
            }
        }

        for (_entity, mut cupboard, interactable) in &mut cupboards {
            if interactable.colliding {
                let message = if cupboard.teapots > 0 {
                    if teapot.is_empty() {
                        cupboard.teapots -= 1;
                        commands.entity(player_entity).insert(TeaPot::default());
                        format!("You take a teapot ({} left).", cupboard.teapots)
                    } else {
                        "You're already carrying a teapot.".to_string()
                    }
                } else {
                    "No teapots remaining.".to_string()
                };
                status_events.send(StatusEvent::timed_message(
                    player_entity,
                    message.to_string(),
                    DEFAULT_EXPIRY,
                ));
                return;
            }
        }

        for (customer_entity, _customer, mut affection, interactable) in &mut customers {
            if interactable.colliding {
                if !teapot.is_empty() {
                    let teapot = teapot.single();

                    if teapot.steeped_at.is_some() {
                        commands.entity(player_entity).remove::<TeaPot>();
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

        for (_entity, cat, interactable, mut affection) in &mut cat {
            if interactable.colliding {
                let (reaction, message) = petting_reaction(&cat, &affection);
                status_events.send(StatusEvent::timed_message(
                    player_entity,
                    message,
                    DEFAULT_EXPIRY,
                ));

                affection.react(reaction);
            }
        }

        for (_entity, interactable) in &kettles {
            if !interactable.colliding {
                continue;
            }
            if teapot.is_empty() {
                status_events.send(StatusEvent::timed_message(
                    player_entity,
                    "You need a teapot to use the kettle.".to_owned(),
                    DEFAULT_EXPIRY,
                ));
                continue;
            }

            let mut teapot = teapot.single_mut();

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
                player_entity,
                message,
                DEFAULT_EXPIRY,
            ));
        }
    }
}
