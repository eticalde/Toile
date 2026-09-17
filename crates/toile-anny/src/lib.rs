//! The Anny body: a real human mesh baked once from CC0 `MakeHuman`/MPFB2
//! data (see `assets/PROVENANCE.txt`) and morphed here at runtime by a
//! phenotype (sex, age, build, muscle, height, proportions) — Toile's one
//! and only body producer.

/// The asset's binary format: the writer the `toile-cli` baker calls and the
/// reader this crate uses, kept in one module so the two cannot drift apart.
///
/// See [`asset::encode`]'s doc for the exact byte layout.
pub mod asset;
/// The 20 catalogue measurements, read directly off a generated mesh.
///
/// Walks the rings and paths [`asset`] bakes alongside the template — the
/// honest half of the promise: the phenotype shapes the body, this says how far
/// it actually lands from the tape.
pub mod measure;
mod mesh;
mod normals;
/// The phenotype: what drives the mesh, and the math that turns it into the
/// per-row weights the asset's deltas are summed with.
pub mod phenotype;
/// Writing a number into a row moves the body: a fixed-iteration secant
/// solve per lever (or per tied pair), cheap because it only ever touches
/// the vertices the lever it is solving actually moves.
pub mod solve;
mod station;

pub use mesh::{BodyMesh, body_mesh};
pub use station::Station;

/// The shape data this build generates bodies from: the embedded asset's
/// format version, and the fingerprint of its payload.
///
/// What a cache of anything derived from a body has to be keyed by besides
/// the phenotype and the levers. The version moves when the layout changes
/// and the fingerprint when the CC0 source or the baker does, so a re-baked
/// asset cannot be mistaken for the one an old entry was computed from.
///
/// # Panics
/// If the embedded bytes carry no readable header, which would mean
/// `assets/body.bin` was hand-edited: the shipped file is written by the
/// baker and read back by this crate's own tests.
pub fn asset_stamp() -> (u32, u64) {
    asset::stamp(mesh::ASSET).expect("the shipped asset carries its header")
}
