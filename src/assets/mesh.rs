//! Engine-agnostic CPU mesh plus the file-loading stub.

use bevy::asset::RenderAssetUsages;
use bevy::math::{Mat3, Mat4, Vec3};
use bevy::mesh::{Indices, PrimitiveTopology};
use std::path::{Path, PathBuf};

/// Mesh file formats the asset pipeline is expected to deliver.
///
/// None of them is decoded in v0.1; the enum exists so that the error surface and the call
/// sites are already in their final shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshFormat {
    /// glTF 2.0 (`.gltf` / `.glb`) – the primary target for AI-generated assets.
    Gltf,
    /// Wavefront OBJ – common export from photogrammetry tools.
    Obj,
}

impl MeshFormat {
    /// Detects the format from a path's extension (case-insensitive).
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "gltf" | "glb" => Some(Self::Gltf),
            "obj" => Some(Self::Obj),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Gltf => "glTF",
            Self::Obj => "OBJ",
        }
    }
}

/// Errors returned by [`Mesh::load_from_file`].
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("unsupported mesh format for {0}")]
    UnsupportedFormat(PathBuf),
    #[error("could not read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{format} loading is not implemented yet (v0.1 stub): {path}")]
    NotImplemented { format: &'static str, path: PathBuf },
    #[error("invalid mesh: {0}")]
    Invalid(String),
}

/// An indexed triangle mesh with per-vertex position, normal and linear-RGBA colour.
///
/// This is the hand-off type between the content pipeline and the runtime: procedural code,
/// and later the AI/photogrammetry pipeline, produce a `Mesh`; the game converts it into a
/// GPU mesh with [`Mesh::to_bevy_mesh`]. Keeping our own type means pipeline tooling and tests
/// never depend on the renderer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    /// Counter-clockwise triangle list.
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads a mesh from disk.
    ///
    /// **v0.1 stub.** The format is detected and the file must exist, but decoding is not
    /// implemented: supported formats return [`AssetError::NotImplemented`]. Callers should
    /// fall back to procedural geometry (see [`crate::assets::procedural`]).
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Mesh, AssetError> {
        let path = path.as_ref();
        let format = MeshFormat::from_path(path)
            .ok_or_else(|| AssetError::UnsupportedFormat(path.to_path_buf()))?;
        std::fs::metadata(path).map_err(|source| AssetError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        // TODO(v0.2): decode glTF via the `gltf` crate / Bevy's glTF loader and convert here.
        Err(AssetError::NotImplemented {
            format: format.name(),
            path: path.to_path_buf(),
        })
    }

    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Checks internal consistency: matching attribute lengths, whole triangles, in-range indices.
    pub fn validate(&self) -> Result<(), AssetError> {
        let n = self.positions.len();
        if self.normals.len() != n || self.colors.len() != n {
            return Err(AssetError::Invalid(format!(
                "attribute length mismatch: {n} positions, {} normals, {} colors",
                self.normals.len(),
                self.colors.len()
            )));
        }
        if !self.indices.len().is_multiple_of(3) {
            return Err(AssetError::Invalid(
                "index count not a multiple of 3".into(),
            ));
        }
        if let Some(bad) = self.indices.iter().find(|&&i| i as usize >= n) {
            return Err(AssetError::Invalid(format!(
                "index {bad} out of range ({n} vertices)"
            )));
        }
        Ok(())
    }

    /// Appends `other` transformed by `transform` (normals use the inverse-transpose).
    pub fn append_transformed(&mut self, other: &Mesh, transform: Mat4) {
        let base = self.positions.len() as u32;
        let normal_matrix = Mat3::from_mat4(transform).inverse().transpose();
        self.positions.extend(
            other
                .positions
                .iter()
                .map(|&p| transform.transform_point3(Vec3::from(p)).to_array()),
        );
        self.normals.extend(other.normals.iter().map(|&n| {
            (normal_matrix * Vec3::from(n))
                .normalize_or_zero()
                .to_array()
        }));
        self.colors.extend_from_slice(&other.colors);
        self.indices.extend(other.indices.iter().map(|i| i + base));
    }

    /// Axis-aligned bounds `(min, max)`, or `None` if empty.
    pub fn bounds(&self) -> Option<(Vec3, Vec3)> {
        let mut it = self.positions.iter().map(|&p| Vec3::from(p));
        let first = it.next()?;
        Some(it.fold((first, first), |(lo, hi), p| (lo.min(p), hi.max(p))))
    }

    /// Converts to a Bevy render mesh (positions, normals, vertex colours, u32 indices).
    pub fn to_bevy_mesh(&self) -> bevy::mesh::Mesh {
        bevy::mesh::Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(bevy::mesh::Mesh::ATTRIBUTE_POSITION, self.positions.clone())
        .with_inserted_attribute(bevy::mesh::Mesh::ATTRIBUTE_NORMAL, self.normals.clone())
        .with_inserted_attribute(bevy::mesh::Mesh::ATTRIBUTE_COLOR, self.colors.clone())
        .with_inserted_indices(Indices::U32(self.indices.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("owg-test-{}-{name}", std::process::id()));
        std::fs::write(&path, b"placeholder").unwrap();
        path
    }

    #[test]
    fn format_detection_is_case_insensitive() {
        assert_eq!(
            MeshFormat::from_path(Path::new("a/car.GLB")),
            Some(MeshFormat::Gltf)
        );
        assert_eq!(
            MeshFormat::from_path(Path::new("car.gltf")),
            Some(MeshFormat::Gltf)
        );
        assert_eq!(
            MeshFormat::from_path(Path::new("scan.obj")),
            Some(MeshFormat::Obj)
        );
        assert_eq!(MeshFormat::from_path(Path::new("car.fbx")), None);
        assert_eq!(MeshFormat::from_path(Path::new("no_extension")), None);
    }

    #[test]
    fn load_from_file_rejects_unknown_extension() {
        let err = Mesh::load_from_file("car.fbx").unwrap_err();
        assert!(matches!(err, AssetError::UnsupportedFormat(_)), "{err}");
    }

    #[test]
    fn load_from_file_reports_missing_file() {
        let err = Mesh::load_from_file("definitely/not/here/car.glb").unwrap_err();
        assert!(matches!(err, AssetError::Io { .. }), "{err}");
    }

    #[test]
    fn load_from_file_is_a_stub_for_supported_formats() {
        for name in ["car.glb", "car.gltf", "scan.obj"] {
            let path = temp_file(name);
            let err = Mesh::load_from_file(&path).unwrap_err();
            std::fs::remove_file(&path).ok();
            match err {
                AssetError::NotImplemented { path: p, .. } => assert_eq!(p, path),
                other => panic!("expected NotImplemented for {name}, got {other}"),
            }
        }
    }

    #[test]
    fn validate_catches_bad_indices_and_lengths() {
        let mut m = Mesh {
            positions: vec![[0.0; 3]; 3],
            normals: vec![[0.0, 1.0, 0.0]; 3],
            colors: vec![[1.0; 4]; 3],
            indices: vec![0, 1, 2],
        };
        assert!(m.validate().is_ok());
        m.indices = vec![0, 1, 3];
        assert!(m.validate().is_err());
        m.indices = vec![0, 1];
        assert!(m.validate().is_err());
        m.indices = vec![0, 1, 2];
        m.colors.pop();
        assert!(m.validate().is_err());
    }

    #[test]
    fn append_transformed_offsets_indices_and_moves_vertices() {
        let tri = Mesh {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            colors: vec![[1.0; 4]; 3],
            indices: vec![0, 1, 2],
        };
        let mut m = tri.clone();
        m.append_transformed(&tri, Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0)));
        assert_eq!(m.indices, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(m.positions[3], [5.0, 0.0, 0.0]);
        assert!(m.validate().is_ok());
        let (lo, hi) = m.bounds().unwrap();
        assert_eq!(lo, Vec3::ZERO);
        assert_eq!(hi, Vec3::new(6.0, 1.0, 0.0));
    }

    #[test]
    fn converts_to_bevy_mesh() {
        let m = crate::assets::procedural::cuboid(Vec3::ONE, [1.0; 4]);
        let bm = m.to_bevy_mesh();
        assert_eq!(bm.count_vertices(), m.vertex_count());
        assert_eq!(bm.indices().map(|i| i.len()), Some(m.indices.len()));
    }
}
