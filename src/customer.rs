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
use crate::personality::{Personality, Personalities};
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
            .add_system(run_looking_for_chair)
            .add_system(run_moving_to_chair)
            .add_system(run_waiting_for_tea)
            .add_system(run_drinking_tea)
            .add_system(run_leave)
            .add_system(customer_spawner)
            .add_system(spawn_customer_by_door)
            .add_system(interact_with_customers)
            .add_system(persist_affection)
            .add_event::<NewCustomerEvent>();
    }
}

impl Customer {
    fn conversation(&self) -> Vec<String> {
        return vec![
            "You: Welcome to Sereni Tea!".to_owned(),
            format!("{:?}: Thank you.", self.personality),
            format!("{:?}: I would like the {}, please.", self.personality, self.expected.name),
            "You: Coming right up!.".to_owned(),
        ];
    }

    fn tea_delivery(&self, teapot: &TeaPot) -> (Reaction, Vec<String>) {
        let hint = teapot
            .ingredients
            .iter()
            .max_by_key(|(_ingredient, amount)| *amount)
            .unwrap()
            .0;

        let mut conversation = vec![
            "You: Here's your tea.".to_owned(),
            format!("{:?}: Oh, thank you!", self.personality),
            format!("{:?}: Is that a hint of {:?}?", self.personality, hint),
            "You: Enjoy!".to_owned(),
        ];
        let recipe_ingredients = HashMap::from_iter(self.expected.ingredients.clone().into_iter());
        let reaction = if recipe_ingredients != teapot.ingredients {
            conversation.push("Customer: Wait a minute! This isn't what I ordered.".to_owned());
            Reaction::Negative
        } else {
            conversation.push("Customer: This is exactly what I was hoping for.".to_owned());
            Reaction::Positive
        };
        (reaction, conversation)
    }
}

#[derive(Component, Deref, DerefMut)]
struct State<T>(pub T);

#[derive(Component)]
struct LookingForChair;

#[derive(Component)]
struct MovingToChair;

#[derive(Component)]
struct WaitingForTea;

#[derive(Component)]
struct DrinkingTea(Timer);

#[derive(Component)]
struct Leaving;

fn run_looking_for_chair(
    customers: Query<(
        Entity, Option<&PathfindTarget>
    ),(
        With<Customer>, With<State<LookingForChair>>, Without<Paused>
    )>,
    chairs: Query<(Entity, &Transform, &HasSize), With<Chair>>,
    mut commands: Commands,
) {
    for (customer_entity, target) in &customers {
        if target.is_none() {
            let mut rng = rand::thread_rng();
            let chair_entity = chairs
                .iter()
                .map(|(entity, _, _)| entity)
                .choose(&mut rng)
                .unwrap();
            commands.entity(customer_entity)
                .insert(PathfindTarget::new(chair_entity, true));
        } else {
            commands.entity(customer_entity)
                .remove::<State<LookingForChair>>()
                .insert(State(MovingToChair));
        }
    }
}

fn run_moving_to_chair(
    mut customers: Query<(
        Entity, Option<&PathfindTarget>, &Transform, &HasSize, &mut Facing,
    ), (
        With<Customer>, With<State<MovingToChair>>, Without<Paused>
    )>,
    chairs: Query<(&Transform, &HasSize), With<Chair>>,
    props: Query<&Transform, (With<Prop>, With<Movable>)>,
    map: Res<Map>,
    mut commands: Commands,
) {
    for (customer_entity, target, transform, sized, mut facing) in &mut customers {
        if target.is_some() {
            continue;
        }

        // Verify that we made it to a chair and didn't just give up.
        let current_pos = transform_to_map_pos(&transform, &map, &sized.size);
        let mut on_chair = false;
        for (chair_transform, chair_size) in &chairs {
            let chair_pos = transform_to_map_pos(&chair_transform, &map, &chair_size.size);
            if chair_pos == current_pos {
                on_chair = true;
                break;
            }
        }

        let mut command_builder = commands.entity(customer_entity);
        command_builder.remove::<State<MovingToChair>>();
        if !on_chair {
            command_builder.insert(State(LookingForChair));
            continue;
        }

        command_builder.insert(State(WaitingForTea));

        // Ensure the customer is facing an appropriate direction for a table,
        // not just the last one they were moving.
        let dirs = [
            FacingDirection::Up,
            FacingDirection::Down,
            FacingDirection::Left,
            FacingDirection::Right
        ];
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
}

fn run_waiting_for_tea(
    mut customers: Query<(
        Entity, Option<&Holding>, &Facing, &mut AnimationData
    ), (
        With<Customer>, With<State<WaitingForTea>>, Without<Paused>
    )>,
    mut commands: Commands,
) {
    for (customer_entity, holding, facing, mut animation) in &mut customers {
        let anim_state = standing_conversion(facing.0);
        if !animation.is_current(anim_state) {
            animation.set_current(anim_state);
        }
        if holding.is_some() {
            commands.entity(customer_entity)
                .remove::<State<WaitingForTea>>()
                .insert(State(DrinkingTea(
                    Timer::new(Duration::from_secs(5), TimerMode::Once)
                )));
        }
    }
}

fn run_drinking_tea(
    mut customers: Query<(
        Entity, &Facing, &mut AnimationData, &mut State<DrinkingTea>
    ), (
        With<Customer>, Without<Paused>,
    )>,
    doors: Query<Entity, With<Door>>,
    mut commands: Commands,
    mut drop_events: EventWriter<DropHeldEntity>,
    time: Res<Time>,
) {
    for (customer_entity, facing, mut animation, mut state) in &mut customers {
        let anim_state = standing_conversion(facing.0);
        if !animation.is_current(anim_state) {
            animation.set_current(anim_state);
        }
        state.0.0.tick(time.delta());
        if !state.0.0.finished() {
            continue;
        }

        commands.entity(customer_entity)
            .remove::<State<DrinkingTea>>()
            .insert(State(Leaving));

        drop_events.send(DropHeldEntity {
            holder: customer_entity,
        });

        let mut rng = rand::thread_rng();
        let door_entity = doors.iter().choose(&mut rng).unwrap();
        commands.entity(customer_entity)
            .insert(PathfindTarget::new(door_entity, true));
    }
}

fn run_leave(
    customers: Query<(
        Entity, Option<&PathfindTarget>
    ), (
        With<State<Leaving>>, With<Customer>, Without<Paused>
    )>,
    mut commands: Commands,
) {
    for (customer_entity, target) in &customers {
        if target.is_none() {
            commands.entity(customer_entity).despawn();
        }
    }
}

fn persist_affection(
    customers: Query<(&Customer, &Affection), Changed<Affection>>,
    mut personalities: ResMut<Personalities>,
) {
    for (customer, affection) in &customers {
        let data = personalities.data.get_mut(&customer.personality).unwrap();
        data.affection = affection.clone();
    }
}

#[derive(Component)]
pub struct Customer {
    pub expected: TeaRecipe,
    pub personality: Personality,
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
    mut personalities: ResMut<Personalities>,
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

        let personality = Personality::generate_random();
        let personality_data = personalities.data.get_mut(&personality).unwrap();
        let affection = personality_data.affection.clone();
        personality_data.visits += 1;

        commands.spawn((
            Customer {
                expected: menu.teas.iter().choose(&mut rng).cloned().unwrap(),
                personality,
            },
            affection,
            Facing(FacingDirection::Down),
            State(LookingForChair),
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
    mut game_state: ResMut<bevy::prelude::State<GameState>>,
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

                    let (reaction, conversation) = customer.tea_delivery(&teapot);
                    affection.react(reaction);
                    game_state.set(GameState::Dialog).unwrap();
                    show_message_box(customer_entity, &mut commands, conversation, &asset_server);
                    return;
                }
            }
        }

        game_state.set(GameState::Dialog).unwrap();
        show_message_box(customer_entity, &mut commands, customer.conversation(), &asset_server);
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
