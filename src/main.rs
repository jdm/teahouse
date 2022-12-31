use bevy::prelude::*;
use crate::animation::animate_sprite;
use crate::cat::{run_cat, interact_with_cat};
use crate::customer::{run_customer, customer_spawner, spawn_customer_by_door, interact_with_customers, NewCustomerEvent};
use crate::debug::{DebugSettings, debug_keys};
use crate::dialog::{run_dialog, exit_dialog};
use crate::entity::setup;
use crate::interaction::{highlight_interactable, keyboard_input, PlayerInteracted};
use crate::map::{read_map, MAP};
use crate::message_line::{update_status_line, StatusEvent};
use crate::movable::move_movables;
use crate::pathfinding::{
    PathingGrid, update_pathing_grid, pathfind_to_target
};
use crate::tea::{
    interact_with_stash, interact_with_cupboards, interact_with_kettles, interact_with_teapot
};

mod animation;
mod cat;
mod customer;
mod debug;
mod dialog;
mod entity;
mod geom;
mod interaction;
mod map;
mod message_line;
mod movable;
mod pathfinding;
mod tea;

fn main() {
    // When building for WASM, print panics to the browser console
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }

    let map = read_map(MAP);

    let mut app = App::new();
    app
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_startup_system(setup)
        .add_state(GameState::InGame)
        .add_system(update_pathing_grid)
        .add_system(move_movables)
        .insert_resource(map)
        .add_system(run_customer)
        .add_system(pathfind_to_target.after(update_pathing_grid).before(move_movables))
        .add_system_set(
            SystemSet::on_update(GameState::InGame)
                .with_system(keyboard_input.before(move_movables))
                .with_system(debug_keys)
                .with_system(highlight_interactable)
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
        .add_system(update_status_line)
        .add_system(customer_spawner)
        .add_system(spawn_customer_by_door)
        .add_system(animate_sprite)
        .add_system(interact_with_stash)
        .add_system(interact_with_cupboards)
        .add_system(interact_with_customers)
        .add_system(interact_with_cat)
        .add_system(interact_with_kettles)
        .add_system(interact_with_teapot)
        .add_event::<StatusEvent>()
        .add_event::<NewCustomerEvent>()
        .add_event::<PlayerInteracted>()
        .init_resource::<PathingGrid>()
        .init_resource::<DebugSettings>();

    #[cfg(target_arch = "wasm32")]
    {
        app.add_system(handle_browser_resize);
    }

    app.run();
}

#[cfg(target_arch = "wasm32")]
fn handle_browser_resize(mut windows: ResMut<Windows>) {
    let window = windows.get_primary_mut().unwrap();
    let wasm_window = web_sys::window().unwrap();
    let (target_width, target_height) = (
        wasm_window.inner_width().unwrap().as_f64().unwrap() as f32,
        wasm_window.inner_height().unwrap().as_f64().unwrap() as f32,
    );
    if window.width() != target_width || window.height() != target_height {
        window.set_resolution(target_width, target_height);
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum GameState {
    InGame,
    Dialog,
}
