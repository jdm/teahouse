use bevy::prelude::*;
use bevy_ecs_tilemap::TilemapPlugin;
use crate::animation::AnimationPlugin;
use crate::cat::CatPlugin;
use crate::customer::CustomerPlugin;
use crate::debug::DebugPlugin;
use crate::dialog::DialogPlugin;
use crate::entity::setup;
use crate::interaction::InteractionPlugin;
use crate::map::MapPlugin;
use crate::message_line::MessageLinePlugin;
use crate::movable::MovablePlugin;
use crate::pathfinding::PathfindingPlugin;
use crate::player::PlayerPlugin;
use crate::tea::TeaPlugin;

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
mod player;
mod tea;

fn main() {
    // When building for WASM, print panics to the browser console
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }

    let mut app = App::new();
    app
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugin(TilemapPlugin)
        .add_plugin(TeaPlugin)
        .add_plugin(CustomerPlugin)
        .add_plugin(CatPlugin)
        .add_plugin(InteractionPlugin)
        .add_plugin(AnimationPlugin)
        .add_plugin(MessageLinePlugin)
        .add_plugin(DebugPlugin)
        .add_plugin(PathfindingPlugin)
        .add_plugin(DialogPlugin)
        .add_plugin(MovablePlugin)
        .add_plugin(MapPlugin)
        .add_plugin(PlayerPlugin)
        .add_state(GameState::InGame)
        //.add_startup_system(setup.after(crate::map::setup_map))
        .add_system(bevy::window::close_on_esc);

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
