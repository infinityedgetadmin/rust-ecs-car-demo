//! Keyboard → [`DriveInput`].

use crate::components::{DriveInput, Player, RunPhase, RunState};
use crate::physics::DriveCommand;
use bevy::app::AppExit;
use bevy::prelude::*;

/// Maps held directional keys to a drive command. Opposing keys cancel out.
pub fn drive_command_from_keys(forward: bool, back: bool, left: bool, right: bool) -> DriveCommand {
    let axis = |pos: bool, neg: bool| f32::from(u8::from(pos)) - f32::from(u8::from(neg));
    DriveCommand {
        throttle: axis(forward, back),
        steer: axis(left, right),
    }
}

/// Reads WASD (and arrow keys) into the player's [`DriveInput`]. Input is ignored once the
/// run is finished so the car coasts to a stop at B.
pub fn read_drive_input(
    keys: Res<ButtonInput<KeyCode>>,
    run: Res<RunState>,
    mut players: Query<&mut DriveInput, With<Player>>,
) {
    let any = |codes: [KeyCode; 2]| keys.any_pressed(codes);
    let command = if matches!(run.phase, RunPhase::Finished(_)) {
        DriveCommand::default()
    } else {
        drive_command_from_keys(
            any([KeyCode::KeyW, KeyCode::ArrowUp]),
            any([KeyCode::KeyS, KeyCode::ArrowDown]),
            any([KeyCode::KeyA, KeyCode::ArrowLeft]),
            any([KeyCode::KeyD, KeyCode::ArrowRight]),
        )
    };
    for mut input in &mut players {
        input.0 = command;
    }
}

/// Quits on Escape.
pub fn exit_on_escape(keys: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_mapping() {
        let c = drive_command_from_keys(true, false, true, false);
        assert_eq!((c.throttle, c.steer), (1.0, 1.0));
        let c = drive_command_from_keys(false, true, false, true);
        assert_eq!((c.throttle, c.steer), (-1.0, -1.0));
        assert_eq!(
            drive_command_from_keys(true, true, true, true),
            DriveCommand::default()
        );
        assert_eq!(
            drive_command_from_keys(false, false, false, false),
            DriveCommand::default()
        );
    }
}
