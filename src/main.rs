use bevy::prelude::*;
use crate::cat::run_cat;
use crate::customer::run_customer;
use crate::debug::debug_keys;
use crate::dialog::{run_dialog, exit_dialog};
use crate::entity::setup;
use crate::interaction::{highlight_interactable, keyboard_input};
use crate::map::{read_map, MAP};
use crate::movable::move_movables;
use crate::pathfinding::{
    PathingGrid, update_pathing_grid, pathfind_to_target
};

mod cat;
mod customer;
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
        .add_system(move_movables)
        .insert_resource(map)
        .add_system(highlight_interactable)
        .add_system(run_customer)
        .add_system(pathfind_to_target.after(update_pathing_grid).before(move_movables))
        .add_system_set(
            SystemSet::on_update(GameState::InGame)
                .with_system(keyboard_input.before(move_movables))
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
        .init_resource::<PathingGrid>()
        .run();
}


#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum GameState {
    InGame,
    Dialog,
}
