//! Asset interface: the seam between the content pipeline and the runtime.
//!
//! *Code is machinery, generated output is content.* The long-term plan is for an AI /
//! photogrammetry pipeline to emit meshes and textures that the game consumes through
//! [`Mesh::load_from_file`]. In v0.1 that function is a stub and all geometry comes from
//! [`procedural`].

pub mod mesh;
pub mod procedural;

pub use mesh::{AssetError, Mesh, MeshFormat};
