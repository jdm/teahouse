use bevy::prelude::*;
use bevy::sprite::collide_aabb::collide;
use crate::GameState;
use crate::entity::{Item, Facing, FacingDirection};
use crate::geom::{TILE_SIZE, HasSize};
use crate::movable::Movable;
use crate::message_line::{DEFAULT_EXPIRY, StatusEvent};
use crate::player::{Holding, Player, SPEED};

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_system(pick_up_item)
            .add_system(mirror_carried_item)
            .add_system(auto_pick_up_item)
            .add_system(transfer)
            .add_system(drop)
            .add_event::<PlayerInteracted>()
            .add_event::<TransferHeldEntity>()
            .add_event::<DropHeldEntity>()
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

#[derive(Component)]
pub struct AutoPickUp;


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
    pub held_entity: Option<Entity>,
}

fn do_pick_up_item(
    commands: &mut Commands,
    player: Entity,
    (item, mut transform): (Entity, &mut Transform),
) {
    commands.entity(player).add_child(item);
    transform.translation = Vec2::ZERO.extend(transform.translation.z);
    commands.entity(player).insert(Holding {
        entity: item,
    });

    // Ensure that carried items aren't considered when checking passability.
    commands.entity(item).remove::<Movable>();
}

fn auto_pick_up_item(
    mut items: Query<(Entity, &mut Transform), (With<Item>, Added<AutoPickUp>)>,
    mut commands: Commands,
    player: Query<Entity, With<Player>>
) {
    for (entity, mut transform) in &mut items {
        let player = player.single();
        commands.entity(entity).remove::<AutoPickUp>();
        do_pick_up_item(
            &mut commands,
            player,
            (entity, &mut transform)
        );
    }
}

fn pick_up_item(
    mut player_interacted_events: EventReader<PlayerInteracted>,
    player_holding: Query<&Holding, With<Player>>,
    mut items: Query<&mut Transform, With<Item>>,
    mut commands: Commands,
) {
    // If the player is already holding an item, they cannot pick up another one.
    if !player_holding.is_empty() {
        return;
    }

    for event in player_interacted_events.iter() {
        let mut transform = match items.get_mut(event.interacted_entity) {
            Ok(result) => result,
            Err(_) => continue,
        };

        do_pick_up_item(
            &mut commands,
            event.player_entity,
            (event.interacted_entity, &mut transform)
        );
    }
}

fn keyboard_input(
    keys: Res<Input<KeyCode>>,
    mut q: Query<(Entity, &mut Movable, &mut Facing), With<Player>>,
    mut interacted_events: EventWriter<PlayerInteracted>,
    mut drop_events: EventWriter<DropHeldEntity>,
    interactables: Query<(Entity, &Interactable)>,
    player_holding: Query<Option<&Holding>, With<Player>>,
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
        let holding = player_holding.single();

        let mut interacting = false;
        for (entity, interactable) in &interactables {
            if !interactable.colliding {
                continue;
            }
            interacting = true;
            interacted_events.send(PlayerInteracted {
                player_entity,
                interacted_entity: entity,
                held_entity: holding.as_ref().map(|h| h.entity),
            });
        }

        // Drop if didn't end up interacting with anything.
        if !interacting && holding.is_some() {
            drop_events.send(DropHeldEntity {
                holder: player_entity,
            });
        }
    }
}

pub struct DropHeldEntity {
    pub holder: Entity
}

fn drop(
    mut events: EventReader<DropHeldEntity>,
    holder: Query<&Holding>,
    mut held_transform: Query<(&mut Transform, &GlobalTransform, &HasSize)>,
    mut commands: Commands,
) {
    for event in events.iter() {
        let held_entity = match holder.get(event.holder) {
            Ok(holder) => holder.entity,
            Err(_) => continue,
        };

        commands.entity(held_entity).remove_parent();
        commands.entity(event.holder).remove::<Holding>();
        let (mut transform, global_transform, sized) = held_transform.get_mut(held_entity).unwrap();
        transform.translation = global_transform.translation();

        // Ensure that dropped items are considered for passability.
        commands.entity(held_entity).insert(Movable {
            size: Vec2::new(
                sized.size.width as f32 * TILE_SIZE,
                sized.size.height as f32 * TILE_SIZE,
            ),
            ..default()
        });
    }
}

pub struct TransferHeldEntity {
    pub holder: Entity,
    pub receiver: Entity,
}

fn transfer(
    mut events: EventReader<TransferHeldEntity>,
    holder: Query<&Holding>,
    facing: Query<&Facing>,
    mut held_transform: Query<&mut Transform>,
    mut commands: Commands,
) {
    for event in events.iter() {
        let held_entity = match holder.get(event.holder) {
            Ok(holder) => holder.entity,
            Err(_) => continue,
        };
        commands.entity(event.receiver).add_child(held_entity);
        commands.entity(event.receiver).insert(Holding {
            entity: held_entity,
        });
        commands.entity(event.holder).remove::<Holding>();
        let holder_facing = facing.get(event.receiver).unwrap();
        let offset = holder_facing.0.offset();
        let mut transform = held_transform.get_mut(held_entity).unwrap();
        //FIXME: awkward. maybe a system for automatically setting transform of new
        //       held entities? also setting the sprite direction based on facing dir?
        transform.translation = Vec3::new(
            offset.0 as f32 * TILE_SIZE,
            offset.1 as f32 * TILE_SIZE,
            transform.translation.z
        );
    }
}
