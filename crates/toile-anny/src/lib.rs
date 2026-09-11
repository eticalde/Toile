//! The Anny body: a neutral adult human mesh baked once from CC0
//! `MakeHuman`/MPFB2 data (see `assets/PROVENANCE.txt`) and read back here at
//! zero runtime cost, as a second producer of `BodyMesh` alongside
//! `toile_body`'s procedural loft.

/// The asset's binary format: the writer the `toile-cli` baker calls and the
/// reader this crate uses, kept in one module so the two cannot drift apart.
///
/// The layout is a small fixed header followed by three flat arrays:
///
/// ```text
/// offset  bytes  field
/// 0       8      magic: b"TOANNY01"
/// 8       4      format version, u32 LE
/// 12      4      vertex count, u32 LE
/// 16      4      triangle count, u32 LE
/// 20      8      FNV-1a hash of everything from offset 28 on, u64 LE
/// 28      12*V   positions: V vertex xyz triples, f32 LE, metres, y-up
/// ...     12*T   indices: T triangle index triples, u32 LE
/// ...     V      stations: one Station tag per vertex, u8
/// ```
pub mod asset;
mod mesh;
mod normals;

pub use mesh::{BodyMesh, body_mesh};
