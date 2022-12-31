use bevy::prelude::*;
use bevy::sprite::collide_aabb::collide;
use crate::GameState;
use crate::entity::{Player, SPEED};
use crate::geom::HasSize;
use crate::movable::Movable;
use crate::message_line::{DEFAULT_EXPIRY, StatusEvent};

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<PlayerInteracted>()
            .add_system_set(
                SystemSet::on_update(GameState::InGame)
                    .with_system(keyboard_input)
                    .with_system(highlight_interactable)
            );
    }
}

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

fn highlight_interactable(
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
    pub player_entity: Entity,
    pub interacted_entity: Entity,
}

fn keyboard_input(
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
