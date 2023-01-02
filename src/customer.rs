use bevy::prelude::*;
use crate::GameState;
use crate::animation::{AtlasAnimationData, AnimationData, AnimData};
use crate::dialog::show_message_box;
use crate::entity::{
    Chair, Door, Reaction, Paused, Affection, Facing, FacingDirection, Prop
};
use crate::geom::{MapSize, map_to_screen, transform_to_map_pos, HasSize, TILE_SIZE};
use crate::interaction::{PlayerInteracted, TransferHeldEntity, DropHeldEntity, Interactable};
use crate::map::Map;
use crate::menu::{Menu, TeaRecipe};
use crate::movable::Movable;
use crate::pathfinding::PathfindTarget;
use crate::player::Holding;
use crate::tea::TeaPot;
use rand::seq::IteratorRandom;
use rand::Rng;
use std::collections::HashMap;
use std::default::Default;
use std::time::Duration;

pub struct CustomerPlugin;

const SPEED: f32 = 40.0;

impl Plugin for CustomerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_startup_system(init_texture)
            .add_system(run_customer)
            .add_system(customer_spawner)
            .add_system(spawn_customer_by_door)
            .add_system(interact_with_customers)
            .add_event::<NewCustomerEvent>();
    }
}

fn conversation(recipe: &TeaRecipe) -> Vec<String> {
    return vec![
        "You: Welcome to Sereni Tea!".to_owned(),
        "Customer: Thank you.".to_owned(),
        format!("Customer: I would like the {}, please.", recipe.name),
        "You: Coming right up!.".to_owned(),
    ];
}

fn tea_delivery(teapot: &TeaPot, recipe: &TeaRecipe) -> (Reaction, Vec<String>) {
    let hint = teapot
        .ingredients
        .iter()
        .max_by_key(|(_ingredient, amount)| *amount)
        .unwrap()
        .0;

    let mut conversation = vec![
        "You: Here's your tea.".to_owned(),
        "Customer: Oh, thank you!".to_owned(),
        format!("Customer: Is that a hint of {:?}?", hint),
        "You: Enjoy!".to_owned(),
    ];
    let recipe_ingredients = HashMap::from_iter(recipe.ingredients.clone().into_iter());
    let reaction = if recipe_ingredients != teapot.ingredients {
        conversation.push("Customer: Wait a minute! This isn't what I ordered.".to_owned());
        Reaction::Negative
    } else {
        conversation.push("Customer: This is exactly what I was hoping for.".to_owned());
        Reaction::Positive
    };
    (reaction, conversation)
}

fn run_customer(
    mut q: Query<(Entity, &mut Customer, Option<&PathfindTarget>, Option<&Holding>, &Transform, &mut Facing, &HasSize, &mut AnimationData), Without<Paused>>,
    chairs: Query<(Entity, &Transform, &HasSize), With<Chair>>,
    doors: Query<Entity, With<Door>>,
    props: Query<&Transform, (With<Prop>, With<Movable>)>,
    mut commands: Commands,
    time: Res<Time>,
    mut drop_events: EventWriter<DropHeldEntity>,
    map: Res<Map>,
) {
    for (entity, mut customer, target, holding, transform, mut facing, sized, mut animation) in &mut q {
        let mut move_to = false;
        let mut leave = false;
        let mut sit = false;
        let mut drink = false;
        match customer.state {
            CustomerState::LookingForChair => {
                if target.is_none() {
                    let mut rng = rand::thread_rng();
                    let chair_entity = chairs
                        .iter()
                        .map(|(entity, _, _)| entity)
                        .choose(&mut rng)
                        .unwrap();
                    commands.entity(entity).insert(PathfindTarget::new(chair_entity, true));
                } else {
                    move_to = true;
                }
            }
            CustomerState::MovingToChair => {
                sit = target.is_none();
            }
            CustomerState::WaitingForTea => {
                let anim_state = standing_conversion(facing.0);
                if !animation.is_current(anim_state) {
                    animation.set_current(anim_state);
                }
                drink = holding.is_some();
            }
            CustomerState::DrinkingTea(ref mut timer) => {
                let anim_state = standing_conversion(facing.0);
                if !animation.is_current(anim_state) {
                    animation.set_current(anim_state);
                }
                timer.tick(time.delta());
                leave = timer.finished();
            }
            CustomerState::Leaving => {
                if target.is_none() {
                    commands.entity(entity).despawn();
                }
            }
        }

        if move_to {
            customer.state = CustomerState::MovingToChair;
        }

        if sit {
            // Verify that we made it to a chair and didn't just give up.
            let current_pos = transform_to_map_pos(&transform, &map, &sized.size);
            let mut on_chair = false;
            for (_, chair_transform, chair_size) in &chairs {
                let chair_pos = transform_to_map_pos(&chair_transform, &map, &chair_size.size);
                if chair_pos == current_pos {
                    on_chair = true;
                    break;
                }
            }
            if !on_chair {
                customer.state = CustomerState::LookingForChair;
                return;
            }

            customer.state = CustomerState::WaitingForTea;
            // Ensure the customer is facing an appropriate direction for a table,
            // not just the last one they were moving.
            let dirs = [FacingDirection::Up, FacingDirection::Down, FacingDirection::Left, FacingDirection::Right];
            let neighbours = dirs
                .iter()
                .map(|dir| dir.adjust_pos(&current_pos))
                .collect::<Vec<_>>();
            for prop_transform in &props {
                let prop_pos = transform_to_map_pos(&prop_transform, &map, &sized.size);
                if let Some(idx) = neighbours.iter().position(|pos| *pos == prop_pos) {
                    facing.0 = dirs[idx];
                    break;
                }
            }
        }

        if drink {
            customer.state = CustomerState::DrinkingTea(Timer::new(Duration::from_secs(5), TimerMode::Once));
        }

        if leave {
            customer.state = CustomerState::Leaving;

            drop_events.send(DropHeldEntity {
                holder: entity,
            });

            let mut rng = rand::thread_rng();
            let door_entity = doors.iter().choose(&mut rng).unwrap();
            commands.entity(entity).insert(PathfindTarget::new(door_entity, true));
        }
    }
}

pub enum CustomerState {
    LookingForChair,
    MovingToChair,
    WaitingForTea,
    DrinkingTea(Timer),
    Leaving,
}

#[derive(Component)]
pub struct Customer {
    pub state: CustomerState,
    pub expected: TeaRecipe,
}

pub struct SpawnerState {
    customer_timer: Timer
}

impl Default for SpawnerState {
    fn default() -> Self {
        Self {
            customer_timer: create_customer_timer()
        }
    }
}

const MIN_SPAWN_TIME: u64 = 30;
const MAX_SPAWN_TIME: u64 = 60;

fn create_customer_timer() -> Timer {
    let mut rng = rand::thread_rng();
    let secs = rng.gen_range(MIN_SPAWN_TIME..MAX_SPAWN_TIME);
    Timer::new(Duration::from_secs(secs), TimerMode::Once)
}

#[derive(Copy, Clone)]
enum AnimationState {
    WalkDown = 0,
    WalkRight = 1,
    WalkLeft = 2,
    WalkUp = 3,
    StandDown = 4,
    StandRight = 5,
    StandLeft = 6,
    StandUp = 7,
}

impl From<AnimationState> for usize {
    fn from(state: AnimationState) -> usize {
        state as usize
    }
}

pub struct NewCustomerEvent;

fn spawn_customer_by_door(
    doors: Query<(&Transform, &HasSize), With<Door>>,
    mut events: EventReader<NewCustomerEvent>,
    mut commands: Commands,
    map: Res<Map>,
    texture: Res<CustomerTexture>,
    menu: Res<Menu>,
) {
    let mut rng = rand::thread_rng();
    // FIXME: assume customers are all 1x1 entities.
    let size = MapSize { width: 1, height: 1 };
    for _event in events.iter() {
        let (transform, sized) = doors.iter().next().unwrap();
        let door_pos = transform_to_map_pos(&transform, &map, &sized.size);
        let screen_rect = map_to_screen(&door_pos, &size, &map);

        let mut translate = transform.translation.clone();
        translate.z = 0.9;
        let screen_size = Vec2::new(screen_rect.w, screen_rect.h);
        let movable = Movable {
            speed: Vec2::ZERO,
            size: screen_size,
            entity_speed: SPEED,
            subtile_max: None,
        };
        let sized = HasSize { size };
        let transform = Transform::from_translation(translate);

        let sprite = SpriteSheetBundle {
            texture_atlas: texture.0.clone(),
            transform,
            ..default()
        };

        commands.spawn((
            Customer {
                state: CustomerState::LookingForChair,
                expected: menu.teas.iter().choose(&mut rng).cloned().unwrap()
            },
            Affection::default(),
            Facing(FacingDirection::Down),
            AnimationData {
                current_animation: AnimationState::WalkDown.into(),
                facing_conversion,
            },
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
}

fn customer_spawner(
    mut state: Local<SpawnerState>,
    mut customer_events: EventWriter<NewCustomerEvent>,
    time: Res<Time>,
) {
    state.customer_timer.tick(time.delta());
    if state.customer_timer.finished() {
        state.customer_timer = create_customer_timer();
        customer_events.send(NewCustomerEvent);
    }
}

fn interact_with_customers(
    mut player_interacted_events: EventReader<PlayerInteracted>,
    mut transfer_events: EventWriter<TransferHeldEntity>,
    mut customers: Query<(Entity, &Customer, &mut Affection)>,
    mut teapot: Query<&mut TeaPot>,
    asset_server: Res<AssetServer>,
    mut game_state: ResMut<State<GameState>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for event in player_interacted_events.iter() {
        let (customer_entity, customer, mut affection) = match customers.get_mut(event.interacted_entity) {
            Ok(result) => result,
            Err(_) => continue,
        };
        if let Some(held) = event.held_entity {
            if let Ok(mut teapot) = teapot.get_mut(held) {
                if teapot.steeped_at.is_some() {
                    transfer_events.send(TransferHeldEntity {
                        holder: event.player_entity,
                        receiver: customer_entity,
                    });
                    //FIXME: wasm issues
                    teapot.steeped_for = Some(time.last_update().unwrap() - teapot.steeped_at.unwrap());

                    let (reaction, conversation) = tea_delivery(&teapot, &customer.expected);
                    affection.react(reaction);
                    game_state.set(GameState::Dialog).unwrap();
                    show_message_box(customer_entity, &mut commands, conversation, asset_server);
                    return;
                }
            }
        }

        game_state.set(GameState::Dialog).unwrap();
        show_message_box(customer_entity, &mut commands, conversation(&customer.expected), asset_server);
        return;
    }
}

#[derive(Resource)]
struct CustomerTexture(Handle<TextureAtlas>);

fn init_texture(
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut animation_data: ResMut<AtlasAnimationData>,
    mut commands: Commands,
) {
    let texture_handle = asset_server.load("woman.png");
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, Vec2::new(TILE_SIZE, TILE_SIZE), 4, 4, None, None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    animation_data.data.insert(
        texture_atlas_handle.clone(),
        vec![
            AnimData { index: 0, frames: 4, delay: 0.1, },
            AnimData { index: 4, frames: 4, delay: 0.1, },
            AnimData { index: 8, frames: 4, delay: 0.1, },
            AnimData { index: 12, frames: 4, delay: 0.1, },
            AnimData { index: 0, frames: 1, delay: 1., },
            AnimData { index: 4, frames: 1, delay: 1., },
            AnimData { index: 8, frames: 1, delay: 1., },
            AnimData { index: 12, frames: 1, delay: 1., },
        ],
    );
    commands.insert_resource(CustomerTexture(texture_atlas_handle));
}

fn standing_conversion(facing: FacingDirection) -> AnimationState {
    match facing {
        FacingDirection::Up => AnimationState::StandUp,
        FacingDirection::Down => AnimationState::StandDown,
        FacingDirection::Right => AnimationState::StandRight,
        FacingDirection::Left => AnimationState::StandLeft,
    }
}

fn facing_conversion(facing: FacingDirection) -> usize {
    match facing {
        FacingDirection::Up => AnimationState::WalkUp,
        FacingDirection::Down => AnimationState::WalkDown,
        FacingDirection::Right => AnimationState::WalkRight,
        FacingDirection::Left => AnimationState::WalkLeft,
    }.into()
}
