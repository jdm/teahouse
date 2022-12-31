use bevy::prelude::*;
use crate::customer::Customer;
use crate::entity::{Affection, RelationshipStatus, CatBed, Player, Reaction};
use crate::pathfinding::PathfindTarget;
use rand::Rng;
use rand::seq::IteratorRandom;
use std::time::Duration;

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

const MIN_SLEEP_TIME: u64 = 30;
const MAX_SLEEP_TIME: u64 = 60;

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

pub fn run_cat(
    mut cat: Query<(Entity, &mut Cat, Option<&PathfindTarget>, &mut Transform)>,
    cat_bed: Query<(Entity, &CatBed)>,
    humans: Query<Entity, Or<(With<Player>, With<Customer>)>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let (entity, mut cat, target, mut transform) = cat.single_mut();
    let mut find_entity = false;
    let mut find_bed = false;
    let mut sleep = false;
    match cat.state {
        CatState::Sleeping(ref mut timer) => {
            timer.tick(time.delta());
            find_entity = timer.finished();
            transform.scale = Vec2::splat(time.elapsed_seconds().sin() + 0.5).extend(0.);
        }
        CatState::MovingToEntity => find_bed = target.is_none(),
        CatState::MovingToBed => sleep = target.is_none(),
    }

    if find_entity {
        transform.scale = Vec2::splat(1.0).extend(0.);
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
    }
}
