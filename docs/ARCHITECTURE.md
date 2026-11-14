# Architecture (v0.1)

## Principles

- **ECS.** Entities are IDs. Components are plain data with no methods that carry game logic.
  Systems are functions that query components and hold all behaviour.
- **Engine-agnostic core.** `physics` and `assets` do not depend on Bevy's ECS, app or
  renderer. They use only `bevy::math` (glam) types and, for `Mesh::to_bevy_mesh`, the mesh
  types. This keeps them unit-testable and reusable by a future server or asset tool.
- **Code is machinery, content is data.** Geometry enters the game through
  `assets::Mesh`. In v0.1 it comes from `assets::procedural`. Later it will come from
  `Mesh::load_from_file` fed by the AI/photogrammetry pipeline.

## Conventions

- +Y up, forward is -Z, right is +X (Bevy's conventions).
- SI units: metres, seconds, radians, m/s.
- Positive yaw and positive steer turn **left** (counter-clockwise seen from above).
- Vertex colours are linear RGBA.

## Plugins

| Plugin | Contains | Used by |
| --- | --- | --- |
| `GameLogicPlugin` | `Route`, `RunState`, input/reset, vehicle integration, goal check | game, headless tests |
| `GamePlugin` | `GameLogicPlugin` + scene spawn, chase camera, HUD, Esc to quit, sky/ambient | game, smoke test |

## Frame data flow

```
Update
├── GameSet::Input
│   ├── goal::reset_on_key       R  -> Transform/Velocity/Heading/RunState reset, camera snap
│   ├── input::read_drive_input  keys -> DriveInput        (zero once the run is finished)
│   └── input::exit_on_escape    Esc -> AppExit
├── GameSet::Simulate
│   └── movement::drive_vehicles DriveInput + Velocity + Heading + Vehicle
│                                 --physics::step_vehicle--> Transform, Velocity, Heading
└── GameSet::Present
    ├── goal::update_run         timer, arrival at B -> RunState
    ├── camera::follow_player    (after update_run) smoothed chase
    └── goal::update_hud         (after update_run) Text
```

## Vehicle step (`physics::step_vehicle`)

1. Decompose the velocity into forward and lateral components.
2. Apply throttle: accelerate, or brake at `brake_decel` if pushing against the direction of
   travel. Reverse uses `reverse_accel_factor`.
3. Damp forward speed with `apply_friction` (exponential drag, then constant rolling
   resistance, with a rest snap when coasting). Clamp to the forward and reverse speed limits.
4. Change yaw by `steer · max_yaw_rate · clamp(v / full_steer_speed, -1, 1) · dt`.
5. Decay lateral speed by `e^{-lateral_grip·dt}`, rebuild the velocity on the new heading, and
   integrate position (semi-implicit Euler).

## Extending

- **New component:** add plain data to `components.rs`. Don't put logic in `impl` blocks
  beyond `Default`.
- **New behaviour:** add a system in `systems/`, put it in one of the `GameSet`s, and unit-test
  any non-trivial math as a pure function.
- **New procedural asset:** add a function returning `assets::Mesh` in `procedural.rs`, with a
  test that calls `validate()` and checks winding.
- **Real asset loading:** implement decoding inside `Mesh::load_from_file`. Keep the error
  variants. The setup system already falls back to procedural geometry on error.
