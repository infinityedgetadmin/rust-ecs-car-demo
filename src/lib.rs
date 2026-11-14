//! Open World Game, v0.1 vertical slice.
//!
//! Module layout (ECS split):
//! - [`components`] – plain-data components and resources.
//! - [`systems`] – all behaviour, as Bevy systems.
//! - [`physics`] – pure, engine-agnostic vehicle math used by the systems.
//! - [`assets`] – engine-agnostic [`assets::Mesh`], the `load_from_file` stub and procedural
//!   geometry.

pub mod assets;
pub mod components;
pub mod physics;
pub mod systems;

use bevy::prelude::*;
use components::{Route, RunState};
use systems::{camera, goal, input, movement, setup};

/// Sky colour.
pub const SKY: Color = Color::srgb(0.53, 0.72, 0.90);

/// Ordering of the per-frame gameplay systems.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSet {
    /// Read devices into components.
    Input,
    /// Integrate motion.
    Simulate,
    /// React to the new state (goal, camera, HUD).
    Present,
}

/// Simulation-only plugin: no window, rendering or asset spawning. Used by the game and by
/// headless tests.
pub struct GameLogicPlugin;

impl Plugin for GameLogicPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Route>()
            .init_resource::<RunState>()
            .configure_sets(
                Update,
                (GameSet::Input, GameSet::Simulate, GameSet::Present).chain(),
            )
            .add_systems(
                Update,
                (
                    (goal::reset_on_key, input::read_drive_input)
                        .chain()
                        .in_set(GameSet::Input),
                    movement::drive_vehicles.in_set(GameSet::Simulate),
                    goal::update_run.in_set(GameSet::Present),
                ),
            );
    }
}

/// The full game: logic plus scene, camera, HUD and quit handling.
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GameLogicPlugin)
            .insert_resource(ClearColor(SKY))
            .insert_resource(GlobalAmbientLight {
                color: Color::WHITE,
                brightness: 400.0,
                ..default()
            })
            .add_systems(Startup, setup::spawn_world)
            .add_systems(
                Update,
                (
                    input::exit_on_escape.in_set(GameSet::Input),
                    (camera::follow_player, goal::update_hud)
                        .after(goal::update_run)
                        .in_set(GameSet::Present),
                ),
            );
    }
}
