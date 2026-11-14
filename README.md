# Open World Game

An open-source, cross-platform (macOS and Windows) open-world game written in Rust on the
[Bevy](https://bevyengine.org) engine. The long-term target is high-fidelity, realistic
open-world driving. **v0.1 is one small playable slice:** drive a car from point A to point B.

> **Status:** v0.1. All automated checks pass (format, clippy, tests, release build), but the
> game was **not run interactively** while it was being built: the development machine was a
> headless Linux box with no display. The rendering path is covered by a headless smoke test
> (scene spawns, camera and HUD systems run) but has not been checked by eye on macOS or
> Windows yet. Please open an issue if something looks wrong on your machine.

## Vision

*Code is machinery, generated output is content.* The runtime is a plain Rust/ECS engine
layer. Over time the world's content (meshes, textures, eventually whole neighbourhoods)
will come from an AI and photogrammetry pipeline (e.g. Meshy, Tripo, photogrammetry
reconstruction), not from hand-written code. That is how a small team gets realistic
content.

v0.1 proves the seam between the two: the runtime loads assets through a
`Mesh::load_from_file` interface. Today that function is a stub and every object is built
from procedural boxes and cylinders. Once the stub decodes glTF, a generated car can replace
the procedural one with no change to gameplay code.

## What's in v0.1

- A large checkered ground plane with a road and scattered crates for a sense of speed.
- A procedurally generated car made of boxes (body, cabin, bumpers, lights) and cylinders
  (wheels).
- WASD driving: throttle, brake/reverse and speed-dependent steering. Velocity is integrated
  each frame with drag, rolling resistance and lateral grip (friction/damping).
- Point A (green marker, start) and point B (gold marker, goal). A HUD shows distance to B,
  speed and elapsed time, plus your finishing time and best time.
- A smoothed third-person chase camera.
- An asset-loading stub: `assets::Mesh::load_from_file`.

## Build and run

You need a stable Rust toolchain, **1.97.1 or newer** (Bevy 0.20 requires it). Install it from
<https://rustup.rs>. Cargo is the only build tool; no CMake is needed.

### macOS

```sh
xcode-select --install          # once, for the system linker
git clone <this repo> && cd open-world-game
cargo run --release
```

### Windows

1. Install the **Visual Studio Build Tools** with the "Desktop development with C++" workload
   (rustup offers to do this).
2. Then:

```powershell
git clone <this repo>; cd open-world-game
cargo run --release
```

The first build compiles Bevy and takes several minutes. Later builds are incremental.
`cargo run` (debug) also works, and dependencies are still optimised in debug builds so it
stays playable.

### Checks (same as CI)

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
```

Linux is not a supported v0.1 target, but the crate compiles there with the X11 backend so
contributors can run the checks.

## Controls

| Key | Action |
| --- | --- |
| `W` / `↑` | Throttle |
| `S` / `↓` | Brake, then reverse |
| `A` / `←` | Steer left |
| `D` / `→` | Steer right |
| `R` | Reset to point A and restart the timer |
| `Esc` | Quit |

Drive from the green marker (A) to the gold marker (B). The run ends when the car is within
6 m of B.

## Architecture

The project is a single crate (`open_world_game`) with a library and a thin binary. It follows
an ECS split: entities are IDs, components are plain data, systems hold all the logic.

```
src/
  main.rs            window setup + GamePlugin; no logic
  lib.rs             GameLogicPlugin (simulation only) and GamePlugin (+ scene, camera, HUD)
  components.rs      plain data: Player, Vehicle, Velocity, Heading, DriveInput,
                     ChaseCamera, GoalMarker, Hud; resources Route, RunState
  physics.rs         pure vehicle math (no ECS): step_vehicle, apply_friction, ...
  systems/
    input.rs         keyboard -> DriveInput, Esc to quit
    movement.rs      DriveInput + Velocity + Heading -> Transform (calls physics)
    camera.rs        chase camera
    goal.rs          A->B timer, arrival detection, R reset, HUD text
    setup.rs         spawns the world from procedural assets
  assets/
    mesh.rs          engine-agnostic Mesh, Mesh::load_from_file stub, to_bevy_mesh()
    procedural.rs    cuboid, cylinder, checker_plane, car, goal_marker
tests/headless.rs    end-to-end ECS tests with simulated keys and a manual clock
docs/ARCHITECTURE.md more detail: data flow, conventions, how to extend
```

Each frame runs three ordered sets: **Input** (reset, read keys) → **Simulate** (integrate
vehicles) → **Present** (goal check, camera, HUD). See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Design decisions

These calls were made during the v0.1 pass. Each one can be revisited.

1. **Switched from raw wgpu/winit + `bevy_ecs` to the Bevy engine.** The interrupted v0.1 had
   five crates (assets, physics, renderer, game_core, game) and a hand-written wgpu renderer.
   The spec prefers Bevy. Using Bevy removes the custom renderer, window loop and input
   plumbing, and gives us PBR lighting, shadows, UI text and a glTF loader for later. The old
   crates were deleted. Their useful parts (friction math, procedural box/cylinder geometry,
   loader error shapes) were ported.
2. **Pinned `bevy = "=0.20.1"`**, the latest stable release on crates.io when this was written.
   `Cargo.lock` is committed and CI uses `--locked`. MSRV is 1.97.1, matching Bevy.
3. **Bevy default features are off, and we opt in to an explicit list.** This drops audio
   (needs ALSA on Linux) and gamepads (`gilrs`, needs libudev on Linux), which v0.1 does not
   use. All the features we enable are platform-neutral. The only Linux-specific feature,
   `x11`, sits in a `[target.'cfg(target_os = "linux")']` dependency, so macOS and Windows
   builds never enable it. It exists only so the checks compile on Linux.
4. **One crate instead of a workspace.** At this size, splitting into crates mostly costs
   compile time and boilerplate. The ECS split is enforced by modules instead: `components`
   holds data only, `systems` holds the logic, `physics` and `assets` are engine-agnostic.
   `physics` uses only `bevy::math`, which re-exports glam. When a module needs to be shared
   with a server or tool, it can move into its own crate.
5. **Our own `assets::Mesh` type, separate from Bevy's `Mesh`.** It is the hand-off format for
   the content pipeline: positions, normals, linear RGBA vertex colours and u32 indices. It
   has `validate()` and `to_bevy_mesh()`. Pipeline tools and tests can use it without the
   renderer.
6. **`Mesh::load_from_file` stub behaviour:** it detects the format from the extension (`.gltf`,
   `.glb`, `.obj`, case-insensitive). Unknown extensions return `UnsupportedFormat`. Missing
   files return `Io`. Supported, existing files return `NotImplemented`. At startup the game
   tries `assets/models/car.glb` and falls back to the procedural car. A real loader drops
   in without changing any call sites.
7. **Arcade vehicle model, not a rigid-body physics engine.** The steps are:
   - throttle accelerates the car, and pressing against the direction of travel brakes it;
   - forward speed decays with exponential drag (`e^{-k·dt}`) plus constant rolling resistance;
   - speed snaps to zero below a rest threshold, so the car actually stops;
   - steering authority grows with speed, so a parked car can't spin and reversing inverts the
     steering;
   - sideways velocity decays exponentially (grip);
   - position is integrated with semi-implicit Euler.

   This is all the spec asks for, it is deterministic and unit-testable, and it avoids taking
   a physics-engine dependency (Rapier/Avian) before we need collisions.
8. **Variable timestep with a clamp** (`MAX_STEP_SECONDS = 1/20`), running in `Update`. The
   exponential damping forms are frame-rate independent (a unit test compares 30 Hz with
   144 Hz). This avoids fixed-timestep interpolation code for v0.1. Move to `FixedUpdate`
   with interpolation when networking needs determinism.
9. **Heading is stored as an explicit `Heading(f32)` component** rather than derived from the
   `Transform` quaternion. The simulation works in yaw and writes the rotation out.
10. **Coordinate conventions:** +Y up, forward is -Z (Bevy's convention), SI units, and positive
    yaw/steer turns left.
11. **Point A → B is a timed run.** Point B is 6 m wide, about 247 m from A, and off-axis so the
    player has to steer. Input is ignored after arrival so the car coasts to a stop. `R`
    resets, and the session-best time is kept.
12. **Chase camera** sits behind and above the car along its heading, looks slightly ahead,
    and follows with frame-rate-independent exponential smoothing. It snaps on spawn and on
    reset.
13. **Testing strategy.** Pure functions (physics, mesh, procedural geometry, goal logic, key
    mapping, camera placement) have unit tests. `tests/headless.rs` drives the real ECS
    systems with a simulated `ButtonInput<KeyCode>` and a manually advanced clock. It also
    smoke-tests the full `GamePlugin` with `MinimalPlugins` + `AssetPlugin`, without a GPU.
14. **Licence:** MIT, keeping the existing `LICENSE`. The spec allows MIT, Apache-2.0 or dual;
    switching to dual MIT/Apache-2.0 later is straightforward.
15. **CI** runs on `macos-latest` and `windows-latest` (the mandatory platforms). Each runs fmt
    check, clippy with `-D warnings`, tests and a release build, all `--locked`.
16. **No Health component.** The spec lists Health only as an example of plain-data components,
    and nothing in v0.1 can take damage. It will be added when it has a system that uses it.

## Roadmap

These items are **not in v0.1**. They are listed only for direction.

- **v0.2: close the asset loop.** Implement `load_from_file` for glTF (via Bevy's glTF loader or
  the `gltf` crate). Load one AI-generated car (Meshy/Tripo) in place of the procedural one.
  Add textures.
- **Driving feel.** Rigid-body physics and collisions (Avian or Rapier), wheel visuals and
  suspension, and a fixed timestep with interpolation.
- **World.** Streaming terrain and roads, and photogrammetry-sourced props.
- **Contributor pipeline.** Contributors submit raw media (photos and video), which goes
  through AI reconstruction, review, and then becomes a game asset.
- **Economy.** A marketplace with a revenue cut on contributed assets, plus free tiers.
- **Hybrid multiplayer.** An authoritative server for live state, with heavy assets served
  from a CDN.

(Neighbourhood intake has been removed from scope.)

## Risks

- **Scope.** This is really five projects in one: an engine/game, an AI asset pipeline, a
  contributor platform, a marketplace and an online service. Each could absorb the whole
  team. v0.1 deliberately proves only one loop, *AI pipeline → Rust runtime on one object*,
  before anything else is built.
- **Economy and legal.** Paying contributors and taking a marketplace cut brings in payments,
  tax, consumer-protection, money-transmission and moderation obligations in many
  jurisdictions.
- **IP in real-world replication.** Recreating real places, buildings, vehicles and brands can
  infringe trademarks, design rights, copyright (e.g. architecture and artwork in some
  countries) and privacy (people and licence plates in captured media).
- **Server costs.** An authoritative multiplayer server plus CDN-delivered heavy assets cost
  money in proportion to usage, before any revenue arrives.
- **The AI pipeline is unproven.** Current generators produce uneven topology, baked lighting
  and inconsistent scale. Whether their output reaches game quality without heavy manual
  cleanup is the core open question, and v0.1/v0.2 exist to answer it cheaply.

## Known limitations (v0.1)

- Not run interactively during development (see Status). Lighting values, handling and the
  camera are tuned on paper.
- No collisions: the car drives through crates and off the edge of the ground plane.
- The world is flat. There is no suspension or wheel animation, and the wheels are part of the
  single car mesh.
- `Mesh::load_from_file` is a stub and always falls back to procedural geometry.
- Variable timestep (clamped), not fixed-step. This is fine for single-player but not for
  networking.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT. See [LICENSE](LICENSE).
