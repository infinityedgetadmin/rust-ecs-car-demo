//! ECS components and resources: plain data only, no behaviour.
//!
//! Systems in [`crate::systems`] own all logic. Rendering data uses Bevy's own components
//! (`Transform`, `Mesh3d`, `MeshMaterial3d`); the mesh *content* comes from
//! [`crate::assets::Mesh`].

use crate::physics::{DriveCommand, VehicleParams};
use bevy::prelude::*;

/// Marks the entity the keyboard controls.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Player;

/// Handling parameters of a drivable vehicle.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Vehicle {
    pub params: VehicleParams,
}

/// World-space linear velocity in m/s.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Deref, DerefMut)]
pub struct Velocity(pub Vec3);

/// Heading (yaw around +Y) in radians; `0` faces -Z. Kept separately from `Transform` so the
/// simulation never has to decompose a quaternion.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Deref, DerefMut)]
pub struct Heading(pub f32);

/// The current driver command for a vehicle, written by input (or, later, AI / network).
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Deref, DerefMut)]
pub struct DriveInput(pub DriveCommand);

/// Third-person chase-camera settings. The camera follows the [`Player`] entity.
#[derive(Component, Debug, Clone, Copy)]
pub struct ChaseCamera {
    /// Horizontal distance behind the target, metres.
    pub distance: f32,
    /// Height above the target's origin, metres.
    pub height: f32,
    /// How far ahead of the target the camera looks, metres.
    pub look_ahead: f32,
    /// Position-follow stiffness, 1/s (higher = tighter).
    pub stiffness: f32,
}

impl Default for ChaseCamera {
    fn default() -> Self {
        Self {
            distance: 9.0,
            height: 3.6,
            look_ahead: 4.0,
            stiffness: 6.0,
        }
    }
}

/// Which end of the route a marker represents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoalKind {
    /// Point A.
    Start,
    /// Point B.
    Finish,
}

/// A visual route marker.
#[derive(Component, Debug, Clone, Copy)]
pub struct GoalMarker {
    pub kind: GoalKind,
}

/// Marks the on-screen status text.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Hud;

/// The A → B route for the v0.1 slice.
#[derive(Resource, Debug, Clone, Copy)]
pub struct Route {
    /// Point A: where the car spawns.
    pub start: Vec3,
    /// Initial heading at point A.
    pub start_yaw: f32,
    /// Point B: the goal.
    pub finish: Vec3,
    /// Horizontal distance from B that counts as arriving, metres.
    pub finish_radius: f32,
}

impl Default for Route {
    fn default() -> Self {
        Self {
            start: Vec3::ZERO,
            start_yaw: 0.0,
            finish: Vec3::new(60.0, 0.0, -240.0),
            finish_radius: 6.0,
        }
    }
}

/// Phase of the current A → B run.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum RunPhase {
    #[default]
    Driving,
    /// Reached B; holds the final time in seconds.
    Finished(f32),
}

/// Timer and result of the current run.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct RunState {
    pub phase: RunPhase,
    /// Seconds since the run started (frozen once finished).
    pub elapsed: f32,
    /// Best finishing time this session.
    pub best: Option<f32>,
}
