use bevy::prelude::*;
use crate::GameState;
use crate::dialog::show_message_box;
use crate::entity::{
    Chair, Door, Reaction, EntityType, Paused, Affection, Facing, FacingDirection,
    Prop, spawn_sprite,
};
use crate::geom::{MapSize, map_to_screen, transform_to_map_pos, HasSize};
use crate::interaction::PlayerInteracted;
use crate::map::Map;
use crate::movable::Movable;
use crate::pathfinding::PathfindTarget;
use crate::player::Player;
use crate::tea::{SpawnTeapotEvent, TeaPot};
use rand::seq::IteratorRandom;
use rand::Rng;
use std::default::Default;
use std::time::Duration;

pub struct CustomerPlugin;

impl Plugin for CustomerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_system(run_customer)
            .add_system(customer_spawner)
            .add_system(spawn_customer_by_door)
            .add_system(interact_with_customers)
            .add_event::<NewCustomerEvent>();
    }
}

pub fn conversation() -> Vec<String> {
    return vec![
        "You: Welcome to Sereni Tea!".to_owned(),
        "Customer: Thank you.".to_owned(),
        "You: I'll bring you some tea.".to_owned(),
    ];
}

fn tea_delivery(teapot: &TeaPot) -> (Reaction, Vec<String>) {
    let hint = teapot
        .ingredients
        .iter()
        .max_by_key(|(_ingredient, amount)| *amount)
        .unwrap()
        .0;

    let conversation = vec![
        "You: Here's your tea.".to_owned(),
        "Customer: Oh, thank you!".to_owned(),
        format!("Customer: Is that a hint of {:?}?", hint),
        "You: Enjoy!".to_owned(),
    ];
    (Reaction::Positive, conversation)
}

fn run_customer(
    mut q: Query<(Entity, &mut Customer, Option<&PathfindTarget>, Option<&TeaPot>, &Transform, &mut Facing, &HasSize), Without<Paused>>,
    chairs: Query<(Entity, &Transform, &HasSize), With<Chair>>,
    doors: Query<Entity, With<Door>>,
    props: Query<&Transform, (With<Prop>, With<Movable>)>,
    mut commands: Commands,
    time: Res<Time>,
    mut teapot_spawner: EventWriter<SpawnTeapotEvent>,
    map: Res<Map>,
) {
    for (entity, mut customer, target, teapot, transform, mut facing, sized) in &mut q {
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
                drink = teapot.is_some();
            }
            CustomerState::DrinkingTea(ref mut timer) => {
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
            commands.entity(entity).remove::<TeaPot>();
            let mut rng = rand::thread_rng();
            let door_entity = doors.iter().choose(&mut rng).unwrap();
            commands.entity(entity).insert(PathfindTarget::new(door_entity, false));

            let current_pos = transform_to_map_pos(&transform, &map, &sized.size);
            let nearest = facing.0.adjust_pos(&current_pos);
            teapot_spawner.send(SpawnTeapotEvent::dirty(nearest));
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
}

impl Default for Customer {
    fn default() -> Self {
        Self {
            state: CustomerState::LookingForChair,
        }
    }
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

pub struct NewCustomerEvent;

fn spawn_customer_by_door(
    doors: Query<(&Transform, &HasSize), With<Door>>,
    mut events: EventReader<NewCustomerEvent>,
    mut commands: Commands,
    map: Res<Map>,
) {
    for _event in events.iter() {
        let (transform, sized) = doors.iter().next().unwrap();
        let mut door_pos = transform_to_map_pos(&transform, &map, &sized.size);
        door_pos.x += 1;
        // FIXME: assume customers are all 1x1 entities.
        let screen_rect = map_to_screen(&door_pos, &MapSize { width: 1, height: 1 }, &map);

        let mut rng = rand::thread_rng();
        let color = Color::rgb(rng.gen(), rng.gen(), rng.gen());

        spawn_sprite(EntityType::Customer(color), screen_rect, &mut commands);
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
