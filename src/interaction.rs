use bevy::prelude::*;
use bevy::sprite::collide_aabb::collide;
use crate::GameState;
use crate::entity::{SPEED, Item, Facing, FacingDirection};
use crate::geom::{TILE_SIZE, HasSize};
use crate::movable::Movable;
use crate::message_line::{DEFAULT_EXPIRY, StatusEvent};
use crate::player::{Holding, Player};

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_system(pick_up_item)
            .add_system(drop_item)
            .add_system(mirror_carried_item)
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

fn mirror_carried_item(
    held: Query<(&Holding, &Facing), (With<Player>, Changed<Facing>)>,
    mut held_sprite: Query<&mut Sprite>,
) {
    if held.is_empty() {
        return;
    }
    let (held, facing) = held.single();
    let mut held_sprite = held_sprite.get_mut(held.entity).unwrap();
    match facing.0 {
        FacingDirection::Left | FacingDirection::Up => held_sprite.flip_x = false,
        FacingDirection::Right | FacingDirection::Down => held_sprite.flip_x = true,
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

fn drop_item(
    held: Query<&Holding, With<Player>>,
    keys: Res<Input<KeyCode>>,
    mut held_transform: Query<(&mut Transform, &GlobalTransform, &HasSize)>,
    mut commands: Commands,
    player: Query<Entity, With<Player>>,
) {
    if held.is_empty() {
        return;
    }

    if keys.just_released(KeyCode::X) {
        let player = player.single();
        let held = held.single();
        commands.entity(held.entity).remove_parent();
        commands.entity(player).remove::<Holding>();
        let (mut transform, global_transform, sized) = held_transform.get_mut(held.entity).unwrap();
        transform.translation = global_transform.translation();

        // Ensure that dropped items are considered for passability.
        commands.entity(held.entity).insert(Movable {
            size: Vec2::new(
                sized.size.width as f32 * TILE_SIZE,
                sized.size.height as f32 * TILE_SIZE,
            ),
            ..default()
        });
    }
}

fn pick_up_item(
    mut player_interacted_events: EventReader<PlayerInteracted>,
    player_holding: Query<&Holding, With<Player>>,
    mut items: Query<(&Item, &mut Transform)>,
    mut commands: Commands,
) {
    // If the player is already holding an item, they cannot pick up another one.
    if !player_holding.is_empty() {
        return;
    }

    for event in player_interacted_events.iter() {
        let (item, mut transform) = match items.get_mut(event.interacted_entity) {
            Ok(result) => result,
            Err(_) => continue,
        };

        commands.entity(event.player_entity).add_child(event.interacted_entity);
        transform.translation = Vec2::ZERO.extend(transform.translation.z);
        commands.entity(event.player_entity).insert(Holding {
            entity: event.interacted_entity,
            _entity_type: item.0.clone(),
        });

        // Ensure that carried items aren't considered when checking passability.
        commands.entity(event.interacted_entity).remove::<Movable>();
    }
}

fn keyboard_input(
    keys: Res<Input<KeyCode>>,
    mut q: Query<(Entity, &mut Movable, &mut Facing), With<Player>>,
    mut interacted_events: EventWriter<PlayerInteracted>,
    interactables: Query<(Entity, &Interactable)>,
) {
    let (player_entity, mut movable, mut facing) = q.single_mut();

    if keys.pressed(KeyCode::Up) {
        movable.speed.y = SPEED;
        facing.0 = FacingDirection::Up;
    } else if keys.pressed(KeyCode::Down) {
        movable.speed.y = -SPEED;
        facing.0 = FacingDirection::Down;
    }
    if keys.just_released(KeyCode::Up) && movable.speed.y > 0. {
        movable.speed.y = 0.0;
    }
    if keys.just_released(KeyCode::Down) && movable.speed.y < 0. {
        movable.speed.y = 0.0;
    }

    if keys.pressed(KeyCode::Left) {
        movable.speed.x = -SPEED;
        facing.0 = FacingDirection::Left;
    }
    if keys.pressed(KeyCode::Right) {
        movable.speed.x = SPEED;
        facing.0 = FacingDirection::Right;
    }
    if keys.just_released(KeyCode::Left) && movable.speed.x < 0. {
        movable.speed.x = 0.0;
    }
    if keys.just_released(KeyCode::Right) && movable.speed.x > 0. {
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
