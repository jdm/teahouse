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
            .add_system(run_cat)
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

#[derive(Debug)]
pub enum CatState {
    Sleeping(Timer),
    MovingToEntity,
    MovingToBed,
}

#[derive(Component)]
pub struct Cat {
    state: CatState,
}

const MIN_SLEEP_TIME: u64 = 3;
const MAX_SLEEP_TIME: u64 = 15;

fn create_sleep_timer() -> Timer {
    let mut rng = rand::thread_rng();
    let secs = rng.gen_range(MIN_SLEEP_TIME..MAX_SLEEP_TIME);
    Timer::new(Duration::from_secs(secs), TimerMode::Once)
}

impl Default for Cat {
    fn default() -> Self {
        Self {
            state: CatState::Sleeping(create_sleep_timer())
        }
    }
}

pub fn petting_reaction(cat: &Cat, affection: &Affection) -> (Reaction, String) {
    let reaction = match cat.state {
        CatState::Sleeping(..) => Reaction::MajorNegative,
        CatState::MovingToEntity |
        CatState::MovingToBed => Reaction::Positive,
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

fn run_cat(
    mut cat: Query<(Entity, &mut Cat, Option<&PathfindTarget>, &mut AnimationData)>,
    cat_bed: Query<(Entity, &CatBed)>,
    humans: Query<Entity, Or<(With<Player>, With<Customer>)>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let (entity, mut cat, target, mut animation) = cat.single_mut();
    let mut find_entity = false;
    let mut find_bed = false;
    let mut sleep = false;
    match cat.state {
        CatState::Sleeping(ref mut timer) => {
            timer.tick(time.delta());
            find_entity = timer.finished();
            animation.set_current(CatAnimationState::Sit);
        }
        CatState::MovingToEntity => find_bed = target.is_none(),
        CatState::MovingToBed => sleep = target.is_none(),
    }

    if find_entity {
        cat.state = CatState::MovingToEntity;
        let mut rng = rand::thread_rng();
        let human_entity = humans.iter().choose(&mut rng).unwrap();
        commands.entity(entity).insert(PathfindTarget::new(human_entity, false));
    }

    if find_bed {
        cat.state = CatState::MovingToBed;
        let (cat_bed_entity, _) = cat_bed.single();
        commands.entity(entity).insert(PathfindTarget::new(cat_bed_entity, true));
    }

    if sleep {
        cat.state = CatState::Sleeping(create_sleep_timer());
        animation.set_current(CatAnimationState::Sit);
    }
}

fn interact_with_cat(
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
