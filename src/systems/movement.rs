//! Velocity integration with friction/damping for every vehicle.

use crate::components::{DriveInput, Heading, Vehicle, Velocity};
use crate::physics::{VehicleState, rotation_from_yaw, step_vehicle};
use bevy::prelude::*;

/// Upper bound on the simulation step. A long hitch (window drag, breakpoint) is simulated as
/// at most this much time instead of teleporting the car.
pub const MAX_STEP_SECONDS: f32 = 1.0 / 20.0;

/// Advances every vehicle by this frame's delta time.
pub fn drive_vehicles(
    time: Res<Time>,
    mut vehicles: Query<(
        &mut Transform,
        &mut Velocity,
        &mut Heading,
        &DriveInput,
        &Vehicle,
    )>,
) {
    let dt = time.delta_secs().min(MAX_STEP_SECONDS);
    if dt <= 0.0 {
        return;
    }
    for (mut transform, mut velocity, mut heading, input, vehicle) in &mut vehicles {
        let state = VehicleState {
            position: transform.translation,
            yaw: heading.0,
            velocity: velocity.0,
        };
        let next = step_vehicle(state, input.0, &vehicle.params, dt);
        transform.translation = next.position;
        transform.rotation = rotation_from_yaw(next.yaw);
        heading.0 = next.yaw;
        velocity.0 = next.velocity;
    }
}
