use std::path::PathBuf;
use std::{fs, io};

use toile_anny::BodyMesh;
use toile_sim::xpbd::SdfGrid;

use super::Collider;
use super::bake::BAKE_VERSION;
use crate::draft::MeasureSet;
use crate::golden::{FNV_BASIS, FNV_PRIME};

#[cfg(test)]
mod tests;

/// The bytes every entry opens with, so an unrelated file is refused before
/// any length arithmetic runs against it.
const MAGIC: [u8; 4] = *b"TSDF";

/// The layout of the file itself, bumped when the header or the order of the
/// data changes. Only this exact version is read.
const FORMAT_VERSION: u32 = 1;

/// The fixed header: the magic, the three versions, the key, the three
/// counts, the cell and the origin.
const HEADER_LEN: usize = 4 + 4 + 4 + 4 + 8 + 12 + 4 + 12;

/// The extension one baked field is kept under.
const EXT: &str = "tsdf";

/// How many bodies the folder keeps.
///
/// An adult body is tens of megabytes, so "paid once and kept" cannot mean
/// "kept for every body ever fitted". Four covers the one body a product is
/// fitted against plus the handful a person tries on the way to it; past that
/// the least recently written one goes, and baking it again costs half a
/// second.
const ENTRIES: usize = 4;

/// What a baked field is filed under: the body it was baked from.
///
/// The measurements go in through the very fingerprint a library link carries
/// — canonical JSON of the tape and the phenotype, and nothing else — so two
/// people measured alike share one entry, and a single centimetre moved is a
/// different one. The shape data's own fingerprint goes in beside them,
/// because the same tape over a re-baked body is a different body.
pub fn key(measures: &MeasureSet) -> u64 {
    // A body that stores no shape is generated from the default one, so the
    // two bake the byte-identical mesh and must not be filed under two keys:
    // tens of megabytes twice, out of a folder that keeps four bodies. The
    // absent shape is written out here, where the key is made, rather than in
    // the fingerprint itself — a library link is stamped with that, and it
    // cannot move.
    let baked = measures
        .clone()
        .shaped(measures.phenotype.unwrap_or_default());
    let (version, shape_table) = toile_anny::asset_stamp();
    let mut h = FNV_BASIS;
    for b in baked.fingerprint().as_bytes() {
        h = (h ^ u64::from(*b)).wrapping_mul(FNV_PRIME);
    }
    for n in [u64::from(version), shape_table] {
        h = (h ^ n).wrapping_mul(FNV_PRIME);
    }
    h
}

/// Baked fields kept between runs, one file per body.
///
/// The folder is handed in, never looked up: the app passes the platform's
/// data directory and a test passes a scratch one, so no test can reach
/// anyone's real cache. Body measurements are personal data, so what is
/// written here is voxels and a hash — never a name, never a tape — and it
/// never leaves the folder it was given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cache {
    dir: PathBuf,
}

/// Why an entry on disk was not the field that was asked for.
enum Stale {
    /// Not an entry at all, or one that stops before its voxels do.
    Unreadable,
    /// Written by a build whose bake or layout was not this one.
    Version,
    /// A whole entry, for another body.
    Key,
}

impl Cache {
    /// The cache kept in `dir`, which need not exist until the first bake is
    /// filed.
    pub fn at(dir: impl Into<PathBuf>) -> Cache {
        Cache { dir: dir.into() }
    }

    /// The field filed under `key`, when there is one this build can read.
    ///
    /// `mesh` is the body the key was computed from: the entry stores the
    /// voxels and not the body, so the extent a release height is derived from
    /// is measured off the mesh again rather than written down twice.
    ///
    /// Every way an entry can disappoint ends in `None` and a re-bake. A file
    /// from a build with another bake version invalidates the whole folder,
    /// since every entry in it was computed the same way; one that is
    /// truncated, corrupt, or filed under someone else's key is thrown away
    /// on its own, and never read half-way.
    pub fn load(&self, key: u64, mesh: &BodyMesh) -> Option<Collider> {
        let path = self.path(key);
        let bytes = fs::read(&path).ok()?;
        match decode(&bytes, key) {
            Ok(field) => Some(Collider::over(field, mesh)),
            Err(Stale::Version) => {
                self.purge();
                None
            }
            Err(Stale::Unreadable | Stale::Key) => {
                let _ = fs::remove_file(&path);
                None
            }
        }
    }

    /// Files a freshly baked field under `key`.
    ///
    /// The bytes land whole, as a temporary file renamed into place, so a run
    /// that dies mid-write leaves the old entry rather than half a new one.
    ///
    /// # Errors
    /// `io::Error` when the folder cannot be made or the file cannot be
    /// written. A cache that cannot be written is a bake paid again, never a
    /// body the person does not get.
    pub fn store(&self, key: u64, collider: &Collider) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let path = self.path(key);
        let staged = self
            .dir
            .join(format!("{key:016x}.{}.{EXT}.tmp", std::process::id()));
        let written = fs::write(&staged, encode(key, collider.field()))
            .and_then(|()| fs::rename(&staged, &path));
        if written.is_err() {
            let _ = fs::remove_file(&staged);
        }
        written?;
        self.evict();
        Ok(())
    }

    /// The file a key names.
    fn path(&self, key: u64) -> PathBuf {
        self.dir.join(format!("{key:016x}.{EXT}"))
    }

    /// Every entry this build wrote, newest last.
    fn entries(&self) -> Vec<(std::time::SystemTime, PathBuf)> {
        let Ok(read) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut found: Vec<(std::time::SystemTime, PathBuf)> = read
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == EXT))
            .filter_map(|path| {
                let when = path.metadata().and_then(|meta| meta.modified()).ok()?;
                Some((when, path))
            })
            .collect();
        found.sort();
        found
    }

    /// Throws away every entry, for a folder written by a build whose bake was
    /// not this one.
    fn purge(&self) {
        for (_, path) in self.entries() {
            let _ = fs::remove_file(path);
        }
    }

    /// Keeps the folder to [`ENTRIES`] bodies, dropping the least recently
    /// written first.
    fn evict(&self) {
        let found = self.entries();
        for (_, path) in found.iter().take(found.len().saturating_sub(ENTRIES)) {
            let _ = fs::remove_file(path);
        }
    }
}

/// The three versions an entry has to agree with to be read at all.
fn versions() -> [u32; 3] {
    [FORMAT_VERSION, toile_anny::asset_stamp().0, BAKE_VERSION]
}

/// Packs a field into an entry's bytes.
///
/// ```text
/// offset  bytes  field
/// 0       4      magic: b"TSDF"
/// 4       4      format version, u32 LE
/// 8       4      mesher version, u32 LE
/// 12      4      bake version, u32 LE
/// 16      8      key, u64 LE
/// 24      12     dims, three u32 LE
/// 36      4      cell, f32 LE, metres
/// 40      12     origin, three f32 LE, metres
/// 52      4*N    data, f32 LE, in the order (k * ny + j) * nx + i
/// ```
fn encode(key: u64, grid: &SdfGrid) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN + grid.data.len() * 4);
    out.extend_from_slice(&MAGIC);
    for version in versions() {
        out.extend_from_slice(&version.to_le_bytes());
    }
    out.extend_from_slice(&key.to_le_bytes());
    for n in grid.dims {
        out.extend_from_slice(&(n as u32).to_le_bytes());
    }
    out.extend_from_slice(&grid.cell.to_le_bytes());
    for c in grid.origin {
        out.extend_from_slice(&c.to_le_bytes());
    }
    for f in &grid.data {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out
}

/// Unpacks an entry's bytes back into the field they carry.
fn decode(bytes: &[u8], key: u64) -> Result<SdfGrid, Stale> {
    if bytes.len() < HEADER_LEN || bytes[0..4] != MAGIC {
        return Err(Stale::Unreadable);
    }
    let word = |at: usize| u32::from_le_bytes(std::array::from_fn(|i| bytes[at + i]));
    if [word(4), word(8), word(12)] != versions() {
        return Err(Stale::Version);
    }
    if u64::from_le_bytes(std::array::from_fn(|i| bytes[16 + i])) != key {
        return Err(Stale::Key);
    }
    let dims = [0, 1, 2].map(|c| word(24 + 4 * c) as usize);
    // Two samples an axis is the floor, and the length check below cannot
    // stand in for it: a header claiming a grid one sample thick agrees with
    // an empty payload, and the trilinear read steps to `i + 1` off the end of
    // the very data it was just proved consistent with.
    if dims.iter().any(|&n| n < 2) {
        return Err(Stale::Unreadable);
    }
    // Checked, because these three numbers come off a file: a corrupt header
    // must not be able to ask for an allocation before anything reads it, and
    // the four bytes a sample takes are part of that same arithmetic.
    let wanted = dims[0]
        .checked_mul(dims[1])
        .and_then(|n| n.checked_mul(dims[2]))
        .and_then(|n| n.checked_mul(4))
        .ok_or(Stale::Unreadable)?;
    let payload = &bytes[HEADER_LEN..];
    if payload.len() != wanted {
        return Err(Stale::Unreadable);
    }
    let read = |at: usize| f32::from_le_bytes(std::array::from_fn(|i| bytes[at + i]));
    Ok(SdfGrid {
        dims,
        cell: read(36),
        origin: [0, 1, 2].map(|c| read(40 + 4 * c)),
        data: payload
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c))
            .collect(),
    })
}
