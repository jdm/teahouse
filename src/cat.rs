use bevy::prelude::*;
use crate::entity::*;
use crate::geom::*;
use crate::pathfinding::*;
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

impl Default for Cat {
    fn default() -> Self {
        Self {
            state: CatState::Sleeping(Timer::new(Duration::from_secs(2), TimerMode::Once))
        }
    }
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
    //println!("{:?} {:?}", cat.state, target);
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
        println!("setting target entity");
        cat.state = CatState::MovingToEntity;
        let mut rng = rand::thread_rng();
        let human_entity = humans.iter().choose(&mut rng).unwrap();
        commands.entity(entity).insert(PathfindTarget {
            target: human_entity,
            next_point: None,
            exact: false,
            current_goal: MapPos { x: 0, y: 0 },
        });
    }

    if find_bed {
        println!("returning to bed");
        cat.state = CatState::MovingToBed;
        let (cat_bed_entity, _) = cat_bed.single();
        commands.entity(entity).insert(PathfindTarget {
            target: cat_bed_entity,
            next_point: None,
            exact: true,
            current_goal: MapPos { x: 0, y: 0 },
        });
    }

    if sleep {
        println!("going to sleep for 5s");
        cat.state = CatState::Sleeping(Timer::new(Duration::from_secs(5), TimerMode::Once));
    }
}
