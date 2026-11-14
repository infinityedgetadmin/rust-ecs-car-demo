//! Point A → point B objective: timer, arrival detection, reset and HUD text.

use crate::components::{
    ChaseCamera, DriveInput, Heading, Hud, Player, Route, RunPhase, RunState, Velocity,
};
use crate::physics::rotation_from_yaw;
use crate::systems::camera::snap_transform;
use bevy::prelude::*;

/// Horizontal (XZ) distance between two points.
pub fn horizontal_distance(a: Vec3, b: Vec3) -> f32 {
    Vec2::new(a.x - b.x, a.z - b.z).length()
}

/// True when `position` is within `radius` of `goal` on the ground plane.
pub fn reached_goal(position: Vec3, goal: Vec3, radius: f32) -> bool {
    horizontal_distance(position, goal) <= radius
}

/// Pure state transition for one frame of a run.
pub fn advance_run(mut run: RunState, dt: f32, arrived: bool) -> RunState {
    if run.phase == RunPhase::Driving {
        run.elapsed += dt.max(0.0);
        if arrived {
            run.phase = RunPhase::Finished(run.elapsed);
            run.best = Some(run.best.map_or(run.elapsed, |b| b.min(run.elapsed)));
        }
    }
    run
}

/// Ticks the run timer and detects arrival at B.
pub fn update_run(
    time: Res<Time>,
    route: Res<Route>,
    mut run: ResMut<RunState>,
    player: Single<&Transform, With<Player>>,
) {
    let arrived = reached_goal(player.translation, route.finish, route.finish_radius);
    let next = advance_run(*run, time.delta_secs(), arrived);
    if next != *run {
        if let (RunPhase::Driving, RunPhase::Finished(t)) = (run.phase, next.phase) {
            info!("Reached point B in {t:.2} s");
        }
        *run = next;
    }
}

/// Press R to put the car back at point A and restart the timer.
pub fn reset_on_key(
    keys: Res<ButtonInput<KeyCode>>,
    route: Res<Route>,
    mut run: ResMut<RunState>,
    player: Single<(&mut Transform, &mut Velocity, &mut Heading, &mut DriveInput), With<Player>>,
    camera: Option<Single<(&mut Transform, &ChaseCamera), Without<Player>>>,
) {
    if !keys.just_pressed(KeyCode::KeyR) {
        return;
    }
    let (mut transform, mut velocity, mut heading, mut input) = player.into_inner();
    *transform =
        Transform::from_translation(route.start).with_rotation(rotation_from_yaw(route.start_yaw));
    *velocity = Velocity::default();
    *heading = Heading(route.start_yaw);
    *input = DriveInput::default();
    *run = RunState {
        best: run.best,
        ..Default::default()
    };
    if let Some(camera) = camera {
        let (mut cam_transform, cam) = camera.into_inner();
        *cam_transform = snap_transform(route.start, route.start_yaw, cam);
    }
}

/// Formats the HUD text. Pure so it can be tested.
pub fn hud_text(run: &RunState, distance_to_b: f32, speed_mps: f32) -> String {
    let best = run
        .best
        .map_or(String::new(), |b| format!("   Best: {b:.1} s"));
    let status = match run.phase {
        RunPhase::Driving => format!(
            "Drive from A (green) to B (gold)\nDistance to B: {distance_to_b:.0} m   \
             Speed: {:.0} km/h   Time: {:.1} s{best}",
            speed_mps.abs() * 3.6,
            run.elapsed
        ),
        RunPhase::Finished(t) => {
            format!("Reached B in {t:.1} s!{best}\nPress R to drive again")
        }
    };
    format!(
        "{status}\nW/S or Up/Down: throttle/brake   A/D or Left/Right: steer   R: reset   Esc: quit"
    )
}

/// Refreshes the on-screen status text.
pub fn update_hud(
    route: Res<Route>,
    run: Res<RunState>,
    player: Single<(&Transform, &Velocity), With<Player>>,
    mut hud: Single<&mut Text, With<Hud>>,
) {
    let (transform, velocity) = *player;
    let distance = horizontal_distance(transform.translation, route.finish);
    let text = hud_text(&run, distance, velocity.length());
    if hud.0 != text {
        hud.0 = text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goal_uses_horizontal_distance() {
        let goal = Vec3::new(10.0, 0.0, 0.0);
        assert!(reached_goal(Vec3::new(7.0, 50.0, 0.0), goal, 3.0));
        assert!(!reached_goal(Vec3::new(6.9, 0.0, 0.0), goal, 3.0));
    }

    #[test]
    fn run_finishes_once_and_tracks_best() {
        let mut run = RunState::default();
        run = advance_run(run, 1.5, false);
        assert_eq!(run.phase, RunPhase::Driving);
        run = advance_run(run, 0.5, true);
        assert_eq!(run.phase, RunPhase::Finished(2.0));
        assert_eq!(run.best, Some(2.0));
        // Timer frozen after finishing.
        run = advance_run(run, 10.0, false);
        assert_eq!(run.elapsed, 2.0);
        // A slower second run keeps the better best.
        let mut second = RunState {
            best: run.best,
            ..Default::default()
        };
        second = advance_run(second, 3.0, true);
        assert_eq!(second.best, Some(2.0));
    }

    #[test]
    fn hud_mentions_state() {
        let run = RunState::default();
        assert!(hud_text(&run, 120.0, 10.0).contains("36 km/h"));
        let done = advance_run(run, 4.0, true);
        assert!(hud_text(&done, 0.0, 0.0).contains("Reached B in 4.0 s"));
    }
}
