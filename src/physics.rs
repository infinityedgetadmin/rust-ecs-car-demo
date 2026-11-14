//! Pure, engine-agnostic vehicle kinematics.
//!
//! Nothing in this module touches the ECS: it is plain functions over plain data so it can be
//! unit-tested deterministically and reused by a future authoritative server. The ECS systems
//! in [`crate::systems::movement`] translate components into a [`VehicleState`], call
//! [`step_vehicle`], and write the result back.
//!
//! Conventions: right-handed, +Y up, a body's local forward axis is -Z (Bevy's convention),
//! SI units (metres, seconds, radians). Positive yaw / positive steer turns **left**
//! (counter-clockwise seen from above).

use bevy::math::{Quat, Vec3};

/// Tunable handling parameters for an arcade-style car.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VehicleParams {
    /// Forward acceleration at full throttle, m/s².
    pub engine_accel: f32,
    /// Deceleration when the throttle opposes the direction of travel, m/s².
    pub brake_decel: f32,
    /// Fraction of `engine_accel` available in reverse.
    pub reverse_accel_factor: f32,
    /// Hard cap on forward speed, m/s.
    pub max_forward_speed: f32,
    /// Hard cap on reverse speed, m/s.
    pub max_reverse_speed: f32,
    /// Yaw rate at full steering lock once the car is at or above `full_steer_speed`, rad/s.
    pub max_yaw_rate: f32,
    /// Speed (m/s) at which steering reaches full authority. Below it steering scales linearly,
    /// so a stationary car cannot spin on the spot.
    pub full_steer_speed: f32,
    /// Exponential drag on forward speed (1/s): `v *= e^(-k·dt)`.
    pub linear_drag: f32,
    /// Constant rolling-resistance deceleration, m/s². Guarantees the car eventually stops.
    pub rolling_resistance: f32,
    /// Exponential decay rate of sideways (sliding) velocity, 1/s. Higher = more grip.
    pub lateral_grip: f32,
    /// Speeds below this snap to exactly zero when there is no throttle.
    pub rest_threshold: f32,
}

impl Default for VehicleParams {
    fn default() -> Self {
        Self {
            engine_accel: 14.0,
            brake_decel: 26.0,
            reverse_accel_factor: 0.5,
            max_forward_speed: 38.0, // ~137 km/h
            max_reverse_speed: 10.0,
            max_yaw_rate: 1.9,
            full_steer_speed: 8.0,
            linear_drag: 0.12,
            rolling_resistance: 1.5,
            lateral_grip: 8.0,
            rest_threshold: 0.05,
        }
    }
}

/// Driver intent for one simulation step. Both axes are clamped to `[-1, 1]` by [`step_vehicle`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DriveCommand {
    /// +1 = full throttle forward, -1 = brake / reverse.
    pub throttle: f32,
    /// +1 = full left, -1 = full right.
    pub steer: f32,
}

/// Minimal kinematic state of a ground vehicle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VehicleState {
    pub position: Vec3,
    /// Heading in radians around +Y. `0` faces -Z.
    pub yaw: f32,
    /// World-space velocity, m/s. Y is ignored (the v0.1 world is flat).
    pub velocity: Vec3,
}

/// Unit forward vector (local -Z) for a heading.
#[inline]
pub fn forward_from_yaw(yaw: f32) -> Vec3 {
    Vec3::new(-yaw.sin(), 0.0, -yaw.cos())
}

/// Unit right vector (local +X) for a heading.
#[inline]
pub fn right_from_yaw(yaw: f32) -> Vec3 {
    Vec3::new(yaw.cos(), 0.0, -yaw.sin())
}

/// Rotation quaternion for a heading.
#[inline]
pub fn rotation_from_yaw(yaw: f32) -> Quat {
    Quat::from_rotation_y(yaw)
}

/// Explicit position integration: `p' = p + v·dt`.
#[inline]
pub fn integrate_position(position: Vec3, velocity: Vec3, dt: f32) -> Vec3 {
    position + velocity * dt
}

/// Applies exponential drag and constant rolling resistance to a signed scalar speed.
///
/// Friction only ever removes speed: it never flips the sign, and results whose magnitude is at
/// or below `rest_threshold` become exactly `0.0`.
pub fn apply_friction(
    speed: f32,
    linear_drag: f32,
    rolling_resistance: f32,
    rest_threshold: f32,
    dt: f32,
) -> f32 {
    let magnitude = speed.abs();
    if magnitude <= rest_threshold {
        return 0.0;
    }
    let damped = magnitude * (-linear_drag * dt).exp();
    let after_rolling = (damped - rolling_resistance * dt).max(0.0);
    if after_rolling <= rest_threshold {
        0.0
    } else {
        after_rolling.copysign(speed)
    }
}

/// Exponentially decays `value` toward zero at `rate` (1/s). Frame-rate independent.
#[inline]
pub fn exp_decay(value: f32, rate: f32, dt: f32) -> f32 {
    value * (-rate * dt).exp()
}

/// Frame-rate-independent interpolation factor for smoothing toward a target with the given
/// stiffness (1/s). Returns a value in `[0, 1)`.
#[inline]
pub fn smoothing_alpha(stiffness: f32, dt: f32) -> f32 {
    1.0 - (-stiffness.max(0.0) * dt.max(0.0)).exp()
}

/// Advances a vehicle by `dt` seconds.
///
/// Model (deliberately arcade, no tyre model):
/// 1. Split velocity into forward and lateral components relative to the heading.
/// 2. Throttle accelerates; throttle against the direction of travel brakes at `brake_decel`.
/// 3. Forward speed is damped by drag + rolling resistance, then clamped to the speed limits.
/// 4. Yaw rate is proportional to steer input and to forward speed (up to `full_steer_speed`),
///    with its sign following the direction of travel, so reversing steers like a real car.
/// 5. Lateral velocity decays exponentially (grip), velocity is re-expressed in the new heading,
///    and position is integrated with the new velocity (semi-implicit Euler).
pub fn step_vehicle(
    state: VehicleState,
    command: DriveCommand,
    p: &VehicleParams,
    dt: f32,
) -> VehicleState {
    if dt <= 0.0 || !dt.is_finite() {
        return state;
    }
    let throttle = command.throttle.clamp(-1.0, 1.0);
    let steer = command.steer.clamp(-1.0, 1.0);

    let forward = forward_from_yaw(state.yaw);
    let right = right_from_yaw(state.yaw);
    let mut forward_speed = state.velocity.dot(forward);
    let mut lateral_speed = state.velocity.dot(right);

    // 2. Engine / brakes.
    let moving_eps = 0.5;
    if throttle > 0.0 {
        if forward_speed < -moving_eps {
            forward_speed = (forward_speed + p.brake_decel * throttle * dt).min(0.0);
        } else {
            forward_speed += p.engine_accel * throttle * dt;
        }
    } else if throttle < 0.0 {
        if forward_speed > moving_eps {
            forward_speed = (forward_speed + p.brake_decel * throttle * dt).max(0.0);
        } else {
            forward_speed += p.engine_accel * p.reverse_accel_factor * throttle * dt;
        }
    }

    // 3. Friction / damping. The rest threshold only applies while coasting, otherwise the car
    //    could never pull away from a standstill.
    let rest = if throttle == 0.0 {
        p.rest_threshold
    } else {
        0.0
    };
    forward_speed = apply_friction(forward_speed, p.linear_drag, p.rolling_resistance, rest, dt);
    forward_speed = forward_speed.clamp(-p.max_reverse_speed, p.max_forward_speed);

    // 4. Steering.
    let authority = if p.full_steer_speed > 0.0 {
        (forward_speed / p.full_steer_speed).clamp(-1.0, 1.0)
    } else {
        forward_speed.signum()
    };
    let yaw = state.yaw + steer * p.max_yaw_rate * authority * dt;

    // 5. Grip, recompose, integrate.
    lateral_speed = exp_decay(lateral_speed, p.lateral_grip, dt);
    if lateral_speed.abs() < 1e-4 {
        lateral_speed = 0.0;
    }
    let velocity = forward_from_yaw(yaw) * forward_speed + right_from_yaw(yaw) * lateral_speed;
    VehicleState {
        position: integrate_position(state.position, velocity, dt),
        yaw,
        velocity,
    }
}

/// Signed speed along the heading, m/s (negative when reversing).
#[inline]
pub fn forward_speed(state: &VehicleState) -> f32 {
    state.velocity.dot(forward_from_yaw(state.yaw))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn run(mut s: VehicleState, cmd: DriveCommand, seconds: f32) -> VehicleState {
        let p = VehicleParams::default();
        let steps = (seconds / DT).round() as usize;
        for _ in 0..steps {
            s = step_vehicle(s, cmd, &p, DT);
        }
        s
    }

    fn moving_forward(speed: f32) -> VehicleState {
        VehicleState {
            velocity: forward_from_yaw(0.0) * speed,
            ..Default::default()
        }
    }

    const GAS: DriveCommand = DriveCommand {
        throttle: 1.0,
        steer: 0.0,
    };
    const COAST: DriveCommand = DriveCommand {
        throttle: 0.0,
        steer: 0.0,
    };
    const BRAKE: DriveCommand = DriveCommand {
        throttle: -1.0,
        steer: 0.0,
    };

    #[test]
    fn basis_vectors_follow_bevy_convention() {
        assert!((forward_from_yaw(0.0) - Vec3::NEG_Z).length() < 1e-6);
        assert!((right_from_yaw(0.0) - Vec3::X).length() < 1e-6);
        let yaw = 0.7;
        assert!((rotation_from_yaw(yaw) * Vec3::NEG_Z - forward_from_yaw(yaw)).length() < 1e-5);
        assert!((rotation_from_yaw(yaw) * Vec3::X - right_from_yaw(yaw)).length() < 1e-5);
    }

    #[test]
    fn integrate_position_is_v_times_dt() {
        let mut p = Vec3::ZERO;
        for _ in 0..60 {
            p = integrate_position(p, Vec3::new(0.0, 0.0, -10.0), DT);
        }
        assert!((p.z + 10.0).abs() < 1e-3, "{p:?}");
    }

    #[test]
    fn throttle_accelerates_forward_along_neg_z() {
        let s = run(VehicleState::default(), GAS, 1.0);
        let v = forward_speed(&s);
        assert!(v > 10.0 && v < 14.0, "speed after 1 s = {v}");
        assert!(
            s.position.z < -5.0,
            "should move toward -Z: {:?}",
            s.position
        );
        assert!(s.position.x.abs() < 1e-4);
    }

    #[test]
    fn speed_is_capped() {
        let p = VehicleParams::default();
        let s = run(VehicleState::default(), GAS, 60.0);
        assert!(forward_speed(&s) <= p.max_forward_speed + 1e-3);
        let s = run(VehicleState::default(), BRAKE, 60.0);
        assert!(forward_speed(&s) >= -p.max_reverse_speed - 1e-3);
        assert!(forward_speed(&s) < -1.0, "holding S at rest should reverse");
    }

    #[test]
    fn coasting_damps_monotonically_to_exact_rest() {
        let p = VehicleParams::default();
        let mut s = moving_forward(30.0);
        let mut last = forward_speed(&s);
        let mut t = 0.0;
        while s.velocity != Vec3::ZERO {
            s = step_vehicle(s, COAST, &p, DT);
            let v = forward_speed(&s);
            assert!(v <= last + 1e-6, "speed increased while coasting");
            assert!(v >= 0.0, "friction reversed direction");
            last = v;
            t += DT;
            assert!(t < 60.0, "car never came to rest");
        }
        assert!(t > 3.0, "damping unrealistically strong: stopped in {t}s");
    }

    #[test]
    fn friction_never_flips_sign_even_with_huge_dt() {
        assert_eq!(apply_friction(3.0, 50.0, 500.0, 0.0, 0.5), 0.0);
        assert_eq!(apply_friction(-3.0, 50.0, 500.0, 0.0, 0.5), 0.0);
        let v = apply_friction(-10.0, 0.1, 1.0, 0.05, DT);
        assert!(v < 0.0 && v > -10.0);
    }

    #[test]
    fn braking_stops_faster_than_coasting() {
        let braked = run(moving_forward(20.0), BRAKE, 1.0);
        let coasted = run(moving_forward(20.0), COAST, 1.0);
        assert!(forward_speed(&braked) < forward_speed(&coasted));
        // Braking from 20 m/s at 26 m/s² takes < 1 s; it must not overshoot into a fast reverse.
        assert!(forward_speed(&braked) > -2.0);
    }

    #[test]
    fn stationary_car_cannot_turn() {
        let s = run(
            VehicleState::default(),
            DriveCommand {
                throttle: 0.0,
                steer: 1.0,
            },
            2.0,
        );
        assert_eq!(s.yaw, 0.0);
        assert_eq!(s.position, Vec3::ZERO);
    }

    #[test]
    fn positive_steer_turns_left_when_driving_forward() {
        let s = run(
            moving_forward(15.0),
            DriveCommand {
                throttle: 1.0,
                steer: 1.0,
            },
            0.5,
        );
        assert!(s.yaw > 0.0);
        assert!(
            s.position.x < 0.0,
            "left of -Z heading is -X: {:?}",
            s.position
        );
    }

    #[test]
    fn steering_inverts_when_reversing() {
        let s = VehicleState {
            velocity: forward_from_yaw(0.0) * -5.0,
            ..Default::default()
        };
        let s = run(
            s,
            DriveCommand {
                throttle: 0.0,
                steer: 1.0,
            },
            0.5,
        );
        assert!(s.yaw < 0.0);
    }

    #[test]
    fn lateral_slide_decays() {
        let s = VehicleState {
            velocity: Vec3::new(10.0, 0.0, 0.0),
            ..Default::default()
        };
        let s = run(s, COAST, 1.0);
        assert!(s.velocity.length() < 0.01, "{:?}", s.velocity);
    }

    #[test]
    fn zero_or_invalid_dt_is_identity() {
        let p = VehicleParams::default();
        let s = moving_forward(12.0);
        assert_eq!(step_vehicle(s, GAS, &p, 0.0), s);
        assert_eq!(step_vehicle(s, GAS, &p, f32::NAN), s);
    }

    #[test]
    fn result_is_frame_rate_independent_enough() {
        let p = VehicleParams::default();
        let mut a = moving_forward(25.0);
        let mut b = a;
        for _ in 0..30 {
            a = step_vehicle(a, COAST, &p, 1.0 / 30.0);
        }
        for _ in 0..144 {
            b = step_vehicle(b, COAST, &p, 1.0 / 144.0);
        }
        assert!((forward_speed(&a) - forward_speed(&b)).abs() < 0.05);
        assert!((a.position - b.position).length() < 0.5);
    }

    #[test]
    fn smoothing_alpha_bounds() {
        assert_eq!(smoothing_alpha(5.0, 0.0), 0.0);
        let a = smoothing_alpha(5.0, DT);
        assert!(a > 0.0 && a < 1.0);
        assert!(smoothing_alpha(5.0, 10.0) < 1.0 + f32::EPSILON);
        assert_eq!(smoothing_alpha(-1.0, DT), 0.0);
        // Two half-steps equal one full step.
        let half = smoothing_alpha(5.0, DT / 2.0);
        assert!(((1.0 - half) * (1.0 - half) - (1.0 - a)).abs() < 1e-6);
    }

    #[test]
    fn exp_decay_halves_at_ln2_over_rate() {
        let v = exp_decay(8.0, 2.0, std::f32::consts::LN_2 / 2.0);
        assert!((v - 4.0).abs() < 1e-4);
    }
}
