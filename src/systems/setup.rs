//! Startup system: builds the v0.1 scene from procedural assets.

use crate::assets::{Mesh, procedural};
use crate::components::{
    ChaseCamera, DriveInput, GoalKind, GoalMarker, Heading, Hud, Player, Route, Vehicle, Velocity,
};
use crate::physics::rotation_from_yaw;
use crate::systems::camera::snap_transform;
use bevy::prelude::*;

/// Optional on-disk car model. If it ever loads, it replaces the procedural car; in v0.1
/// [`Mesh::load_from_file`] is a stub so the procedural car is always used.
pub const CAR_MODEL_PATH: &str = "assets/models/car.glb";

/// Side length of the square ground plane, metres.
pub const GROUND_SIZE: f32 = 800.0;

/// The gameplay (non-visual) components of the player car, spawned at point A.
/// Kept separate from rendering so headless tests can spawn the same car.
pub fn player_car_bundle(route: &Route) -> impl Bundle {
    (
        Name::new("Player car"),
        Player,
        Vehicle::default(),
        Velocity::default(),
        Heading(route.start_yaw),
        DriveInput::default(),
        Transform::from_translation(route.start).with_rotation(rotation_from_yaw(route.start_yaw)),
    )
}

/// Loads the car model through the asset stub, falling back to procedural geometry.
pub fn car_mesh() -> Mesh {
    match Mesh::load_from_file(CAR_MODEL_PATH) {
        Ok(mesh) if mesh.validate().is_ok() => mesh,
        Ok(_) => {
            warn!("{CAR_MODEL_PATH} is invalid; using procedural car");
            procedural::car(&procedural::CarPalette::default())
        }
        Err(err) => {
            info!("Using procedural car ({err})");
            procedural::car(&procedural::CarPalette::default())
        }
    }
}

/// Tiny deterministic hash → `[0, 1)`, used to scatter props without an RNG dependency.
fn hash01(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// Distance from point `p` to the segment `a`–`b` on the XZ plane.
fn distance_to_segment_xz(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let (p, a, b) = (p.xz(), a.xz(), b.xz());
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(f32::EPSILON)).clamp(0.0, 1.0);
    (a + ab * t).distance(p)
}

/// Spawns ground, road, A/B markers, props, the car, lights, camera and HUD.
pub fn spawn_world(
    mut commands: Commands,
    route: Res<Route>,
    mut meshes: ResMut<Assets<bevy::mesh::Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // One white material: all colour comes from vertex colours on our procedural meshes.
    let vertex_colored = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.8,
        ..default()
    });
    let glossy = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.35,
        metallic: 0.1,
        ..default()
    });
    let mut add_mesh = |mesh: Mesh| meshes.add(mesh.to_bevy_mesh());

    // Ground: 10 m checker tiles so motion is easy to read.
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(add_mesh(procedural::checker_plane(
            GROUND_SIZE,
            (GROUND_SIZE / 10.0) as u32,
            [0.16, 0.30, 0.12, 1.0],
            [0.13, 0.25, 0.10, 1.0],
        ))),
        MeshMaterial3d(vertex_colored.clone()),
        Transform::default(),
    ));

    // Road strip from A to B.
    let delta = route.finish - route.start;
    let length = delta.xz().length() + 30.0;
    let road_yaw = (-delta.x).atan2(-delta.z);
    let mid = (route.start + route.finish) * 0.5;
    commands.spawn((
        Name::new("Road"),
        Mesh3d(add_mesh(procedural::cuboid(
            Vec3::new(10.0, 0.04, length),
            [0.05, 0.05, 0.055, 1.0],
        ))),
        MeshMaterial3d(vertex_colored.clone()),
        Transform::from_translation(mid.with_y(0.02)).with_rotation(rotation_from_yaw(road_yaw)),
    ));

    // Markers.
    for (kind, at, color) in [
        (GoalKind::Start, route.start, [0.05, 0.75, 0.15, 1.0]),
        (GoalKind::Finish, route.finish, [1.0, 0.70, 0.05, 1.0]),
    ] {
        let radius = if kind == GoalKind::Finish {
            route.finish_radius
        } else {
            3.0
        };
        commands.spawn((
            Name::new(format!("{kind:?} marker")),
            GoalMarker { kind },
            Mesh3d(add_mesh(procedural::goal_marker(radius, 8.0, color))),
            MeshMaterial3d(vertex_colored.clone()),
            Transform::from_translation(at.with_y(0.0)),
        ));
    }

    // Scattered crates for a sense of speed, kept off the road.
    let crate_mesh = add_mesh(procedural::cuboid(
        Vec3::splat(2.0),
        [0.55, 0.38, 0.20, 1.0],
    ));
    let half = GROUND_SIZE * 0.5 - 10.0;
    for i in 0..220u32 {
        let p = Vec3::new(
            (hash01(i * 3) * 2.0 - 1.0) * half,
            0.0,
            (hash01(i * 3 + 1) * 2.0 - 1.0) * half,
        );
        if distance_to_segment_xz(p, route.start, route.finish) < 12.0 {
            continue;
        }
        let scale = 0.6 + hash01(i * 3 + 2) * 1.6;
        commands.spawn((
            Mesh3d(crate_mesh.clone()),
            MeshMaterial3d(vertex_colored.clone()),
            Transform::from_translation(p.with_y(scale))
                .with_rotation(rotation_from_yaw(hash01(i * 7) * std::f32::consts::TAU))
                .with_scale(Vec3::splat(scale)),
        ));
    }

    // The car.
    commands.spawn((
        player_car_bundle(&route),
        Mesh3d(add_mesh(car_mesh())),
        MeshMaterial3d(glossy),
    ));

    // Lighting.
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY * 2.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::default().looking_to(Vec3::new(-0.4, -1.0, -0.3), Vec3::Y),
    ));

    // Camera.
    let chase = ChaseCamera::default();
    commands.spawn((
        Name::new("Chase camera"),
        Camera3d::default(),
        snap_transform(route.start, route.start_yaw, &chase),
        chase,
    ));

    // HUD.
    commands.spawn((
        Hud,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_in_unit_range_and_deterministic() {
        for i in 0..1000 {
            let h = hash01(i);
            assert!((0.0..1.0).contains(&h));
            assert_eq!(h, hash01(i));
        }
    }

    #[test]
    fn segment_distance() {
        let a = Vec3::ZERO;
        let b = Vec3::new(0.0, 0.0, -10.0);
        assert!((distance_to_segment_xz(Vec3::new(3.0, 5.0, -5.0), a, b) - 3.0).abs() < 1e-5);
        assert!((distance_to_segment_xz(Vec3::new(0.0, 0.0, 4.0), a, b) - 4.0).abs() < 1e-5);
    }

    #[test]
    fn car_mesh_falls_back_to_procedural() {
        let m = car_mesh();
        assert!(m.validate().is_ok());
        assert!(m.triangle_count() > 100);
    }
}
