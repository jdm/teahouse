use bevy::prelude::*;
use crate::entity::{Chair, Door, Reaction};
use crate::pathfinding::PathfindTarget;
use crate::tea::TeaPot;
use rand::seq::IteratorRandom;
use std::default::Default;
use std::time::Duration;

pub fn tea_delivery(teapot: &TeaPot) -> (Reaction, Vec<String>) {
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

pub fn run_customer(
    mut q: Query<(Entity, &mut Customer, Option<&PathfindTarget>, Option<&TeaPot>)>,
    chairs: Query<Entity, With<Chair>>,
    doors: Query<Entity, With<Door>>,
    mut commands: Commands,
    time: Res<Time>,
) {
    for (entity, mut customer, target, teapot) in &mut q {
        let mut move_to = false;
        let mut leave = false;
        let mut sit = false;
        let mut drink = false;
        match customer.state {
            CustomerState::LookingForChair => {
                if target.is_none() {
                    let mut rng = rand::thread_rng();
                    let chair_entity = chairs.iter().choose(&mut rng).unwrap();
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
            customer.state = CustomerState::WaitingForTea;
        }

        if drink {
            println!("customer got teapot");
            customer.state = CustomerState::DrinkingTea(Timer::new(Duration::from_secs(5), TimerMode::Once));
        }

        if leave {
            customer.state = CustomerState::Leaving;
            let mut rng = rand::thread_rng();
            let door_entity = doors.iter().choose(&mut rng).unwrap();
            commands.entity(entity).insert(PathfindTarget::new(door_entity, false));
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
    pub conversation: Vec<String>,
}

impl Default for Customer {
    fn default() -> Self {
        Self {
            state: CustomerState::LookingForChair,
            conversation: vec![],
        }
    }
}
