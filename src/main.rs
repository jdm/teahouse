use bevy::prelude::*;
use crate::cat::run_cat;
use crate::debug::debug_keys;
use crate::dialog::{run_dialog, exit_dialog};
use crate::entity::setup;
use crate::interaction::{highlight_interactable, keyboard_input};
use crate::map::{read_map, MAP};
use crate::movable::{check_for_collisions, move_movable, halt_collisions, CollisionEvent};
use crate::pathfinding::{
    PathfindEvent, PathingGrid, update_pathing_grid, select_pathfinding_targets,
    pathfind, pathfind_to_target,
};

mod cat;
mod debug;
mod dialog;
mod entity;
mod geom;
mod interaction;
mod map;
mod movable;
mod pathfinding;

fn main() {
    let map = read_map(MAP);

    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_state(GameState::InGame)
        .add_system(update_pathing_grid)
        .add_system(check_for_collisions)
        .add_system(halt_collisions.after(check_for_collisions))
        .add_system(move_movable.after(halt_collisions))
        .insert_resource(map)
        .add_system(highlight_interactable)
        .add_system(select_pathfinding_targets)
        .add_system(pathfind.after(update_pathing_grid))
        .add_system(pathfind_to_target.after(update_pathing_grid).before(check_for_collisions))
        .add_system_set(
            SystemSet::on_update(GameState::InGame)
                .with_system(keyboard_input)
                .with_system(debug_keys)
                .with_system(bevy::window::close_on_esc)
        )
        .add_system_set(
            SystemSet::on_update(GameState::Dialog)
                .with_system(run_dialog)
        )
        .add_system_set(
            SystemSet::on_exit(GameState::Dialog)
                .with_system(exit_dialog)
        )
        .add_system(run_cat)
        .add_event::<CollisionEvent>()
        .add_event::<PathfindEvent>()
        .init_resource::<PathingGrid>()
        .run();
}


#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum GameState {
    InGame,
    Dialog,
}
