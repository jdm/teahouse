use bevy::prelude::*;
use bevy::sprite::collide_aabb::collide;
use crate::GameState;
use crate::cat::Cat;
use crate::customer::Customer;
use crate::dialog::show_message_box;
use crate::entity::*;
use crate::geom::HasSize;
use crate::movable::Movable;

#[derive(Component)]
pub struct StatusMessage {
    pub source: Option<Entity>,
}

#[derive(Bundle)]
pub struct StatusMessageBundle {
    pub message: StatusMessage,

    #[bundle]
    pub text: TextBundle,
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

pub fn highlight_interactable(
    player: Query<(&Transform, &Movable), With<Player>>,
    mut interactable: Query<(Entity, &mut Interactable, &Transform, &mut Sprite, &HasSize)>,
    mut status: Query<(&mut StatusMessage, &mut Text)>,
) {
    let (player_transform, player_movable) = player.single();
    let (mut status, mut status_text) = status.single_mut();

    for (entity, mut interactable, transform, mut sprite, size) in interactable.iter_mut() {
        let screen_size = size.screen_size();
        let collision = collide(
            transform.translation,
            Vec2::new(screen_size.0, screen_size.1),
            player_transform.translation,
            player_movable.size * 1.3,
        );
        if collision.is_some() && status.source.is_none() {
            if interactable.previous.is_none() {
                interactable.previous = Some(sprite.color);
                sprite.color = interactable.highlight;
            }
            interactable.colliding = true;

            status.source = Some(entity);
            status_text.sections[0].value = interactable.message.clone();

        } else if collision.is_none() && interactable.previous.is_some() {
            sprite.color = interactable.previous.take().unwrap();
            interactable.colliding = false;

            status.source = None;
            status_text.sections[0].value = "".to_string();
        }
    }
}

pub fn keyboard_input(
    keys: Res<Input<KeyCode>>,
    mut q: Query<(Entity, &mut Player, &mut Movable)>,
    mut interactables: Query<(&mut TeaStash, &Interactable)>,
    mut cupboards: Query<(&mut Cupboard, &Interactable)>,
    customers: Query<(&Customer, &Interactable)>,
    mut cat: Query<(&Interactable, &mut Affection), With<Cat>>,
    kettles: Query<&Interactable, With<Kettle>>,
    mut commands: Commands,
    mut game_state: ResMut<State<GameState>>,
    mut teapot: Query<&mut TeaPot, With<Player>>,
    asset_server: Res<AssetServer>,
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
        for (mut stash, interactable) in &mut interactables {
            if interactable.colliding {
                stash.amount -= 1;
                let amount = player.carrying.entry(stash.ingredient).or_insert(0);
                *amount += 1;
                println!("carrying: {:?}", player.carrying);
                return;
            }
        }

        for (mut cupboard, interactable) in &mut cupboards {
            if interactable.colliding {
                cupboard.teapots -= 1;
                commands.entity(player_entity).insert(TeaPot::default());
                println!("acquired teapot");
                return;
            }
        }

        for (customer, interactable) in &customers {
            if interactable.colliding {
                game_state.set(GameState::Dialog).unwrap();
                show_message_box(&mut commands, customer.conversation.clone(), asset_server);
                return;
            }
        }

        for (interactable, mut affection) in &mut cat {
            if interactable.colliding {
                println!("You pet the cat.");
                affection.react(Reaction::Positive);
            }
        }

        if !teapot.is_empty() {
            let mut teapot = teapot.single_mut();
            for interactable in &kettles {
                if interactable.colliding {
                    println!("You fill the teapot.");
                    teapot.water = 100;
                }
            }
        }
    }
}
