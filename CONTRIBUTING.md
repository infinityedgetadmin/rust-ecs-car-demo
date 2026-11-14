# Contributing

Thanks for helping! v0.1 is small on purpose. Please read the README's Roadmap and Risks
sections before proposing large features.

## Getting set up

Install stable Rust 1.97.1 or newer from <https://rustup.rs>. Before you open a PR, run the
same checks CI runs:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
```

CI runs these checks on macOS and Windows. Both platforms are mandatory: don't add
platform-specific code or features without a `cfg` that keeps the other platforms building.
Don't enable Bevy features that need extra system libraries unless that's discussed first.

## Code style

- Format with `rustfmt` (settings in `rustfmt.toml`, max width 100). Clippy must pass with
  `-D warnings`. `type_complexity` and `too_many_arguments` are allowed crate-wide because
  they are noise for Bevy systems.
- Follow the ECS split described in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md):
  - **Components** (`src/components.rs`) are plain data. Derive `Component`, `Debug`,
    `Clone`, and `Default` where it makes sense. Don't put game logic in methods.
  - **Systems** (`src/systems/`) hold the logic. Put each system in a `GameSet`.
  - **Math** that can be a pure function should be one, with unit tests (see `src/physics.rs`).
- Follow the conventions: +Y up, -Z forward, SI units, positive yaw turns left.
- Write doc comments for public items, and say what units they use.
- Every behaviour change needs a test. Prefer pure-function unit tests. Use
  `tests/headless.rs` for end-to-end ECS behaviour.
- Keep dependencies minimal and pinned. Explain any new dependency in the PR.

## Entity submission guidelines

An *entity* is a gameplay object: a vehicle, a prop, a marker.

1. Define its data as components in `components.rs`, reusing existing ones (`Velocity`,
   `Heading`, `Vehicle`) where you can.
2. Provide a bundle function (like `systems::setup::player_car_bundle`) that holds only the
   gameplay components, so headless tests can spawn the entity without a renderer. Add the
   rendering components (`Mesh3d`, `MeshMaterial3d`) separately.
3. Its visuals must come from an `assets::Mesh`: procedural for now, or loaded once the loader
   exists.
4. Include tests for any new behaviour.

## Asset submission guidelines

The asset pipeline (AI generation, photogrammetry) is on the roadmap and **binary assets are
not accepted in v0.1** while `Mesh::load_from_file` is a stub. When loading lands, assets
will need to meet these requirements:

- **Format:** glTF 2.0 binary (`.glb`) preferred. `.gltf` and `.obj` are also recognised.
- **Orientation and scale:** +Y up, front facing -Z, 1 unit = 1 metre, origin on the ground
  under the object's centre.
- **Geometry:** triangulated, outward-facing counter-clockwise winding, no degenerate
  triangles. A reasonable triangle budget (e.g. a car under 50k triangles).
- **Rights:** you must own the asset or have the right to license it under the project's
  licence. **Don't submit** scans or generations of trademarked vehicles, logos, artworks,
  identifiable people or licence plates, or private property without permission. AI-generated
  assets must name the tool used and confirm its terms allow this use.
- **Provenance:** include a short sidecar note (`<name>.md`) with the source (tool,
  photogrammetry capture, or hand-made), the date, and the licence.

## Pull requests

- Keep PRs focused. Describe what changed and how you tested it, including the OS you ran
  the game on.
- By contributing, you agree that your contributions are licensed under the project's MIT
  licence.
