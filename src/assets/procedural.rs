//! Procedural geometry used by v0.1 in place of authored or AI-generated assets.
//!
//! Everything is boxes and cylinders. Conventions: +Y up, the car's front points toward -Z,
//! units are metres, colours are linear RGBA.

use super::mesh::Mesh;
use bevy::math::{Mat4, Quat, Vec3};
use std::f32::consts::{FRAC_PI_2, TAU};

/// Linear-space RGBA colour.
pub type Rgba = [f32; 4];

fn push_vertex(mesh: &mut Mesh, position: Vec3, normal: Vec3, color: Rgba) {
    mesh.positions.push(position.into());
    mesh.normals.push(normal.into());
    mesh.colors.push(color);
}

/// Axis-aligned box centred on the origin with flat-shaded faces (24 vertices, 12 triangles).
pub fn cuboid(size: Vec3, color: Rgba) -> Mesh {
    let half = size * 0.5;
    let mut mesh = Mesh::new();
    for normal in [
        Vec3::X,
        Vec3::NEG_X,
        Vec3::Y,
        Vec3::NEG_Y,
        Vec3::Z,
        Vec3::NEG_Z,
    ] {
        // Tangents chosen so that u × v == normal → counter-clockwise when seen from outside.
        let v = if normal.y.abs() > 0.5 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        let u = v.cross(normal);
        let (c, u, v) = (normal * half, u * half, v * half);
        let base = mesh.positions.len() as u32;
        for p in [c - u - v, c + u - v, c + u + v, c - u + v] {
            push_vertex(&mut mesh, p, normal, color);
        }
        mesh.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    mesh
}

/// Capped cylinder centred on the origin with its axis along +Y.
pub fn cylinder(radius: f32, height: f32, segments: u32, color: Rgba) -> Mesh {
    let segments = segments.max(3);
    let h = height * 0.5;
    let ring = |i: u32| {
        let a = i as f32 / segments as f32 * TAU;
        Vec3::new(a.cos(), 0.0, a.sin())
    };
    let mut mesh = Mesh::new();

    // Side wall, smooth normals. Vertex pairs (bottom, top) per ring step.
    for i in 0..=segments {
        let dir = ring(i);
        for y in [-h, h] {
            push_vertex(&mut mesh, dir * radius + Vec3::Y * y, dir, color);
        }
    }
    for i in 0..segments {
        let b0 = i * 2;
        let (t0, b1, t1) = (b0 + 1, b0 + 2, b0 + 3);
        mesh.indices.extend_from_slice(&[b0, t0, t1, b0, t1, b1]);
    }

    // Caps, flat normals.
    for (y, normal) in [(h, Vec3::Y), (-h, Vec3::NEG_Y)] {
        let center = mesh.positions.len() as u32;
        push_vertex(&mut mesh, Vec3::Y * y, normal, color);
        for i in 0..=segments {
            push_vertex(&mut mesh, ring(i) * radius + Vec3::Y * y, normal, color);
        }
        for i in 0..segments {
            let (a, b) = (center + 1 + i, center + 2 + i);
            if normal.y > 0.0 {
                mesh.indices.extend_from_slice(&[center, b, a]);
            } else {
                mesh.indices.extend_from_slice(&[center, a, b]);
            }
        }
    }
    mesh
}

/// Flat checkerboard on the XZ plane, centred on the origin, facing +Y.
pub fn checker_plane(size: f32, tiles: u32, color_a: Rgba, color_b: Rgba) -> Mesh {
    let tiles = tiles.max(1);
    let step = size / tiles as f32;
    let origin = -size * 0.5;
    let mut mesh = Mesh::new();
    for tz in 0..tiles {
        for tx in 0..tiles {
            let color = if (tx + tz) % 2 == 0 { color_a } else { color_b };
            let (x0, z0) = (origin + tx as f32 * step, origin + tz as f32 * step);
            let (x1, z1) = (x0 + step, z0 + step);
            let base = mesh.positions.len() as u32;
            for (x, z) in [(x0, z0), (x0, z1), (x1, z1), (x1, z0)] {
                push_vertex(&mut mesh, Vec3::new(x, 0.0, z), Vec3::Y, color);
            }
            mesh.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
    mesh
}

/// Colours for [`car`].
#[derive(Clone, Copy, Debug)]
pub struct CarPalette {
    pub body: Rgba,
    pub cabin: Rgba,
    pub trim: Rgba,
    pub tyre: Rgba,
    pub headlight: Rgba,
    pub taillight: Rgba,
}

impl Default for CarPalette {
    fn default() -> Self {
        Self {
            body: [0.70, 0.06, 0.04, 1.0],
            cabin: [0.04, 0.06, 0.10, 1.0],
            trim: [0.10, 0.10, 0.11, 1.0],
            tyre: [0.02, 0.02, 0.02, 1.0],
            headlight: [1.0, 0.90, 0.50, 1.0],
            taillight: [0.85, 0.01, 0.01, 1.0],
        }
    }
}

/// Wheel radius of [`car`]; the origin sits on the ground under the car's centre.
pub const CAR_WHEEL_RADIUS: f32 = 0.38;

/// A simple car (~2.0 m wide, ~1.5 m tall, ~4.4 m long) made of boxes and cylinder wheels.
pub fn car(palette: &CarPalette) -> Mesh {
    let mut mesh = Mesh::new();
    let mut add = |part: Mesh, transform: Mat4| mesh.append_transformed(&part, transform);
    let at = Mat4::from_translation;

    // Chassis, cabin, roof.
    add(
        cuboid(Vec3::new(1.9, 0.55, 4.2), palette.body),
        at(Vec3::new(0.0, 0.65, 0.0)),
    );
    add(
        cuboid(Vec3::new(1.6, 0.5, 2.0), palette.cabin),
        at(Vec3::new(0.0, 1.17, 0.25)),
    );
    add(
        cuboid(Vec3::new(1.7, 0.08, 2.1), palette.body),
        at(Vec3::new(0.0, 1.45, 0.25)),
    );
    // Bumpers.
    for z in [-2.15, 2.15] {
        add(
            cuboid(Vec3::new(2.0, 0.22, 0.18), palette.trim),
            at(Vec3::new(0.0, 0.45, z)),
        );
    }
    // Lights (front is -Z).
    for x in [-0.65, 0.65] {
        let lamp = Vec3::new(0.4, 0.14, 0.06);
        add(
            cuboid(lamp, palette.headlight),
            at(Vec3::new(x, 0.75, -2.12)),
        );
        add(
            cuboid(lamp, palette.taillight),
            at(Vec3::new(x, 0.75, 2.12)),
        );
    }
    // Wheels: cylinders rotated so their axis runs along X.
    let wheel = cylinder(CAR_WHEEL_RADIUS, 0.32, 20, palette.tyre);
    let hub = cylinder(0.18, 0.34, 12, palette.trim);
    let to_x_axis = Quat::from_rotation_z(FRAC_PI_2);
    for x in [-0.95, 0.95] {
        for z in [-1.35, 1.35] {
            let t = Mat4::from_rotation_translation(to_x_axis, Vec3::new(x, CAR_WHEEL_RADIUS, z));
            add(wheel.clone(), t);
            add(hub.clone(), t);
        }
    }
    mesh
}

/// A goal marker: a flat pad on the ground plus a tall thin pole, centred on the origin.
pub fn goal_marker(radius: f32, pole_height: f32, color: Rgba) -> Mesh {
    let mut mesh = Mesh::new();
    // Pad bottom sits a hair above y = 0 to avoid z-fighting with the ground.
    let pad = cylinder(radius, 0.06, 40, color);
    mesh.append_transformed(&pad, Mat4::from_translation(Vec3::Y * 0.04));
    let pole = cylinder(0.15, pole_height, 12, color);
    mesh.append_transformed(&pole, Mat4::from_translation(Vec3::Y * pole_height * 0.5));
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: Rgba = [1.0; 4];

    /// Every triangle's geometric normal should point the same way as its vertex normals.
    fn assert_outward_winding(mesh: &Mesh) {
        for tri in mesh.indices.as_chunks::<3>().0 {
            let [a, b, c] =
                [tri[0], tri[1], tri[2]].map(|i| Vec3::from(mesh.positions[i as usize]));
            let face = (b - a).cross(c - a);
            let vn = Vec3::from(mesh.normals[tri[0] as usize])
                + Vec3::from(mesh.normals[tri[1] as usize])
                + Vec3::from(mesh.normals[tri[2] as usize]);
            assert!(face.dot(vn) > 0.0, "inward-facing triangle {tri:?}");
        }
    }

    #[test]
    fn cuboid_is_valid_and_sized() {
        let m = cuboid(Vec3::new(2.0, 1.0, 4.0), WHITE);
        m.validate().unwrap();
        assert_eq!(m.vertex_count(), 24);
        assert_eq!(m.triangle_count(), 12);
        assert_eq!(
            m.bounds().unwrap(),
            (Vec3::new(-1.0, -0.5, -2.0), Vec3::new(1.0, 0.5, 2.0))
        );
        assert_outward_winding(&m);
    }

    #[test]
    fn cylinder_is_valid_and_outward() {
        let m = cylinder(0.5, 2.0, 16, WHITE);
        m.validate().unwrap();
        assert_outward_winding(&m);
        let (lo, hi) = m.bounds().unwrap();
        assert!((hi.y - 1.0).abs() < 1e-6 && (lo.y + 1.0).abs() < 1e-6);
        assert!((hi.x - 0.5).abs() < 1e-6);
    }

    #[test]
    fn car_rests_on_ground_and_faces_neg_z() {
        let m = car(&CarPalette::default());
        m.validate().unwrap();
        assert_outward_winding(&m);
        let (lo, hi) = m.bounds().unwrap();
        assert!(
            lo.y.abs() < 1e-4,
            "wheels should touch y=0, min y = {}",
            lo.y
        );
        assert!(hi.z - lo.z > 4.0 && hi.x - lo.x < 2.5);
        // Headlights (front) are at -Z.
        let head = CarPalette::default().headlight;
        let i = m.colors.iter().position(|c| *c == head).unwrap();
        assert!(m.positions[i][2] < -2.0);
    }

    #[test]
    fn checker_plane_alternates() {
        let m = checker_plane(10.0, 2, [1.0; 4], [0.0, 0.0, 0.0, 1.0]);
        m.validate().unwrap();
        assert_outward_winding(&m);
        assert_eq!(m.triangle_count(), 8);
        assert_ne!(m.colors[0], m.colors[4]);
    }

    #[test]
    fn goal_marker_sits_on_ground() {
        let m = goal_marker(3.0, 6.0, WHITE);
        m.validate().unwrap();
        let (lo, hi) = m.bounds().unwrap();
        assert!(lo.y >= 0.0 && (hi.y - 6.0).abs() < 1e-4);
    }
}
