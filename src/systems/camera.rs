//! Third-person chase camera.

use crate::components::{ChaseCamera, Heading, Player};
use crate::physics::{forward_from_yaw, smoothing_alpha};
use bevy::prelude::*;

/// Where the camera wants to be: behind and above the target along its heading.
pub fn chase_position(target: Vec3, yaw: f32, cam: &ChaseCamera) -> Vec3 {
    target - forward_from_yaw(yaw) * cam.distance + Vec3::Y * cam.height
}

/// The point the camera looks at: slightly above and ahead of the target.
pub fn chase_look_target(target: Vec3, yaw: f32, cam: &ChaseCamera) -> Vec3 {
    target + Vec3::Y * 1.0 + forward_from_yaw(yaw) * cam.look_ahead
}

/// Places the camera immediately (no smoothing), e.g. on spawn or reset.
pub fn snap_transform(target: Vec3, yaw: f32, cam: &ChaseCamera) -> Transform {
    Transform::from_translation(chase_position(target, yaw, cam))
        .looking_at(chase_look_target(target, yaw, cam), Vec3::Y)
}

/// Smoothly follows the player with frame-rate-independent exponential smoothing.
pub fn follow_player(
    time: Res<Time>,
    player: Single<(&Transform, &Heading), With<Player>>,
    camera: Single<(&mut Transform, &ChaseCamera), Without<Player>>,
) {
    let (target, heading) = *player;
    let (mut cam_transform, cam) = camera.into_inner();
    let desired = chase_position(target.translation, heading.0, cam);
    let alpha = smoothing_alpha(cam.stiffness, time.delta_secs());
    cam_transform.translation = cam_transform.translation.lerp(desired, alpha);
    cam_transform.look_at(
        chase_look_target(target.translation, heading.0, cam),
        Vec3::Y,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_sits_behind_and_above() {
        let cam = ChaseCamera::default();
        let p = chase_position(Vec3::ZERO, 0.0, &cam);
        // Car faces -Z, so "behind" is +Z.
        assert!((p - Vec3::new(0.0, cam.height, cam.distance)).length() < 1e-5);
        let t = snap_transform(Vec3::ZERO, 0.0, &cam);
        assert!(t.forward().z < 0.0, "camera should look toward -Z");
    }

    #[test]
    fn camera_rotates_with_heading() {
        let cam = ChaseCamera::default();
        let p = chase_position(Vec3::ZERO, std::f32::consts::FRAC_PI_2, &cam);
        // Heading +90° faces -X, so the camera sits at +X.
        assert!(p.x > cam.distance - 1e-4 && p.z.abs() < 1e-4);
    }
}
