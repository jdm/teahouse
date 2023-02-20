use bevy::prelude::*;
use crate::action::ActionPlugin;
use crate::animation::AnimationPlugin;
use crate::bookshelf::BookshelfPlugin;
use crate::cat::CatPlugin;
use crate::customer::CustomerPlugin;
use crate::debug::DebugPlugin;
use crate::dialog::DialogPlugin;
use crate::entity::setup;
use crate::interaction::InteractionPlugin;
use crate::map::MapPlugin;
use crate::menu::MenuPlugin;
use crate::message_line::MessageLinePlugin;
use crate::movable::MovablePlugin;
use crate::pathfinding::PathfindingPlugin;
use crate::personality::PersonalityPlugin;
use crate::player::PlayerPlugin;
use crate::stair::StairPlugin;
use crate::tea::TeaPlugin;
use crate::trigger::TriggerPlugin;

mod action;
mod animation;
mod bookshelf;
mod cat;
mod customer;
mod debug;
mod dialog;
mod entity;
mod geom;
mod interaction;
mod map;
mod menu;
mod message_line;
mod movable;
mod pathfinding;
mod personality;
mod player;
mod stair;
mod tea;
mod trigger;

fn main() {
    // When building for WASM, print panics to the browser console
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }

    let mut app = App::new();
    app
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    window: WindowDescriptor {
                        title: "since i found serenitea...".to_string(),
                        ..default()
                    },
                    ..default()
                })
        )
        //.add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        //.add_plugin(bevy::diagnostic::LogDiagnosticsPlugin::default())
        .add_plugin(TeaPlugin)
        .add_plugin(BookshelfPlugin)
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
        .add_plugin(MenuPlugin)
        .add_plugin(PersonalityPlugin)
        .add_plugin(TriggerPlugin)
        .add_plugin(ActionPlugin)
        .add_state(GameState::Loading)
        .add_plugin(StairPlugin)
        .add_startup_system(setup)
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
    Loading,
    Processing,
    InGame,
    Dialog,
}
