//! The Anny body: a real human mesh baked once from CC0 `MakeHuman`/MPFB2
//! data (see `assets/PROVENANCE.txt`) and morphed here at runtime by a
//! phenotype (sex, age, build, muscle, height, proportions), as a second
//! producer of `BodyMesh` alongside `toile_body`'s procedural loft.

/// The asset's binary format: the writer the `toile-cli` baker calls and the
/// reader this crate uses, kept in one module so the two cannot drift apart.
///
/// See [`asset::encode`]'s doc for the exact byte layout.
pub mod asset;
/// The 20 catalogue measurements, read directly off a generated mesh.
///
/// Walks the rings [`asset`] bakes alongside the template — the honest half
/// of the promise: the phenotype shapes the body, this says how far it
/// actually lands from the tape.
pub mod measure;
mod mesh;
mod normals;
/// The phenotype: what drives the mesh, and the math that turns it into the
/// per-row weights the asset's deltas are summed with.
pub mod phenotype;

pub use mesh::{BodyMesh, body_mesh};
