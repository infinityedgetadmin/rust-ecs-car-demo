//! Binary entry point. All game code lives in the library crate.

use bevy::prelude::*;
use open_world_game::GamePlugin;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Open World Game v0.1".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(GamePlugin)
        .run()
}
