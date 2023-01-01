use bevy::prelude::*;
use crate::animation::AnimationData;
use crate::customer::Customer;
use crate::entity::{Affection, RelationshipStatus, Player, Reaction, Facing, FacingDirection};
use crate::interaction::PlayerInteracted;
use crate::message_line::{DEFAULT_EXPIRY, StatusEvent};
use crate::pathfinding::PathfindTarget;
use rand::Rng;
use rand::seq::IteratorRandom;
use std::time::Duration;

pub struct CatPlugin;

impl Plugin for CatPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_system(interact_with_cat)
            .add_system(run_sleep)
            .add_system(run_follow)
            .add_system(run_bed)
            .add_system(update_animation_state);
    }
}

pub enum CatAnimationState {
    WalkDown = 0,
    WalkRight = 1,
    WalkUp = 2,
    WalkLeft = 3,
    Sit = 4,
}

impl From<CatAnimationState> for usize {
    fn from(state: CatAnimationState) -> usize {
        state as usize
    }
}

#[derive(Component)]
pub struct CatBed;

#[derive(Component)]
pub struct Sleeping(Timer);

#[derive(Component)]
struct MovingToEntity;

#[derive(Component)]
struct MovingToBed;

fn run_sleep(
    mut cat: Query<(Entity, &mut State<Sleeping>, &mut AnimationData), With<Cat>>,
    humans: Query<Entity, Or<(With<Player>, With<Customer>)>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    if cat.is_empty() {
        return;
    }
    let (cat_entity, mut state, mut animation) = cat.single_mut();
    animation.set_current(CatAnimationState::Sit);
    state.0.0.tick(time.delta());
    if state.0.0.finished() {
        commands.entity(cat_entity).remove::<State<Sleeping>>();

        let mut rng = rand::thread_rng();
        let human_entity = humans.iter().choose(&mut rng).unwrap();
        commands.entity(cat_entity).insert(PathfindTarget::new(human_entity, false));
        commands.entity(cat_entity).insert(State(MovingToEntity));
    }
}

fn run_follow(
    cat: Query<(Entity, Option<&PathfindTarget>), (With<Cat>, With<State<MovingToEntity>>)>,
    cat_bed: Query<Entity, With<CatBed>>,
    mut commands: Commands,
) {
    if cat.is_empty() {
        return;
    }
    let (cat_entity, target) = cat.single();
    if target.is_none() {
        commands.entity(cat_entity).remove::<State<MovingToEntity>>();
        let cat_bed_entity = cat_bed.single();
        commands.entity(cat_entity).insert(PathfindTarget::new(cat_bed_entity, true));
        commands.entity(cat_entity).insert(State(MovingToBed));
    }
}

fn run_bed(
    mut cat: Query<(Entity, Option<&PathfindTarget>), (With<Cat>, With<State<MovingToBed>>)>,
    mut commands: Commands,
) {
    if cat.is_empty() {
        return;
    }
    let (cat_entity, target) = cat.single_mut();
    if target.is_none() {
        commands.entity(cat_entity).remove::<State<MovingToEntity>>();
        commands.entity(cat_entity).insert(State(Sleeping(create_sleep_timer())));
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct State<T>(pub T);

impl Default for State<Sleeping> {
    fn default() -> Self {
        Self(Sleeping(create_sleep_timer()))
    }
}

#[derive(Component, Default)]
pub struct Cat;

const MIN_SLEEP_TIME: u64 = 3;
const MAX_SLEEP_TIME: u64 = 15;

fn create_sleep_timer() -> Timer {
    let mut rng = rand::thread_rng();
    let secs = rng.gen_range(MIN_SLEEP_TIME..MAX_SLEEP_TIME);
    Timer::new(Duration::from_secs(secs), TimerMode::Once)
}

fn petting_reaction(is_sleeping: bool, affection: &Affection) -> (Reaction, String) {
    let reaction = if is_sleeping {
        Reaction::MajorNegative
    } else {
        Reaction::Positive
    };
    let message = match affection.status() {
        RelationshipStatus::Angry => "The cat hisses.",
        RelationshipStatus::Neutral => "The cat ignores you.",
        RelationshipStatus::Friendly => "The cat purrs.",
        RelationshipStatus::VeryFriendly => "The cat purrs and rubs against you.",
        RelationshipStatus::Crushing => "The cat purs and headbutts your hand.",
    }.to_string();
    (reaction, message)
}

fn update_animation_state(
    mut cat_animation: Query<(&mut AnimationData, &Facing), (With<Cat>, Changed<Facing>)>,
) {
    if cat_animation.is_empty() {
        return;
    }

    let (mut data, facing) = cat_animation.single_mut();
    let state = match facing.0 {
        FacingDirection::Up => CatAnimationState::WalkUp,
        FacingDirection::Down => CatAnimationState::WalkDown,
        FacingDirection::Right => CatAnimationState::WalkRight,
        FacingDirection::Left => CatAnimationState::WalkLeft,
    };
    data.set_current(state);
}

fn interact_with_cat(
    mut player_interacted_events: EventReader<PlayerInteracted>,
    mut cat: Query<(&mut Affection, Option<&State<Sleeping>>), With<Cat>>,
    mut status_events: EventWriter<StatusEvent>,
) {
    for event in player_interacted_events.iter() {
        let (mut affection, sleeping) = match cat.get_mut(event.interacted_entity) {
            Ok(result) => result,
            Err(_) => continue,
        };
        let (reaction, message) = petting_reaction(sleeping.is_some(), &affection);
        status_events.send(StatusEvent::timed_message(
            event.player_entity,
            message,
            DEFAULT_EXPIRY,
        ));

        affection.react(reaction);
    }
}
