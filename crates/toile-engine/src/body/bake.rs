use toile_anny::BodyMesh;
use toile_sim::xpbd::SdfGrid;

use self::pseudo::Pseudo;

mod band;
mod fill;
mod manifold;
mod pseudo;
#[cfg(test)]
mod tests;

pub use manifold::{Manifold, inspect};

/// The spacing between samples, in metres.
///
/// Five millimetres: fine enough that the gap a garment hangs at is several
/// voxels, coarse enough to be affordable at all. An adult stands with the
/// arms away from the sides, so the box around one is largely air and the
/// field still comes to some forty megabytes. The cost is the cube of this
/// number: at a millimetre the same body would ask for five gigabytes.
pub const CELL: f64 = 0.005;

/// How far either side of the skin the field carries a true distance, in
/// metres.
///
/// The solver only ever asks about a particle it is holding against the
/// body, so exactness two and a half centimetres out is exactness where it
/// is read. Past that the field saturates, and that is what keeps this a
/// narrow band rather than a full distance transform over 3.6 M voxels.
pub const BAND: f64 = 0.025;

/// The version of the bake: bumped whenever the field a given mesh produces
/// changes bits.
///
/// `libm` is pinned exactly, the way `kurbo` is. Its determinism is that it
/// is the same code everywhere, not that it is correctly rounded, so a
/// different version of that code is a different field. A `libm` bump is
/// therefore a bump of this number and regenerated SDF goldens in the same
/// commit — never a silent drift under a cached field.
pub const BAKE_VERSION: u32 = 2;

/// The most samples a bake will allocate.
///
/// A lattice is bounded by the mesh's own bounding box and nothing else, and
/// [`super::Collider::bake`] is public, so a mesh whose positions are
/// millimetres rather than metres asks for a grid of the cube of the extent
/// and dies in the allocator. Sixty-four million is close to four times the
/// largest field this project bakes — the demo sphere's 16.7 M — and 256 MB
/// in `f32`: an unusually large body still bakes, a mesh in the wrong unit is
/// refused for the price of three multiplications.
pub const MAX_VOXELS: usize = 64_000_000;

/// Why a mesh could not be baked.
#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
pub enum BakeError {
    /// The mesh has no triangles to be a surface of.
    #[error("a mesh with no triangle encloses nothing")]
    Empty,
    /// A position is NaN or infinite, so no distance to it is a number.
    #[error("a position is not finite")]
    NotFinite,
    /// The mesh is not the closed, orientable 2-manifold the sign needs.
    #[error("the mesh does not earn a sign: {0:?}")]
    NotClosed(Manifold),
    /// The mesh's extent asks for more samples than a bake will allocate.
    #[error("the mesh asks for a {0:?} grid, past what a bake will allocate")]
    TooLarge([usize; 3]),
}

/// The voxel lattice a bake writes into: where the samples are, and how many.
pub(super) struct Lattice {
    /// Samples along x, y and z.
    pub(super) dims: [usize; 3],
    /// Where the first sample sits.
    pub(super) origin: [f64; 3],
    /// The spacing between samples.
    pub(super) cell: f64,
    /// How far either side of the skin this bake stays true.
    pub(super) band: f64,
}

impl Lattice {
    /// A lattice around a mesh's own bounding box, padded by the band and one
    /// cell.
    ///
    /// That pad is what lets the flood fill seed itself from the outer face
    /// of the box: with it, no sample out there can be within the band of the
    /// surface, so every one of them is known to be outside without asking.
    ///
    /// The origin and the cell are rounded through `f32` before any sample
    /// position is computed from them, because the grid the solver reads
    /// carries them as `f32`: rounding here means the bake and the solver
    /// agree on where a voxel is, rather than by half a micron.
    fn around(positions: &[f32], cell: f64, band: f64) -> Lattice {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for p in positions.as_chunks::<3>().0 {
            for c in 0..3 {
                lo[c] = lo[c].min(f64::from(p[c]));
                hi[c] = hi[c].max(f64::from(p[c]));
            }
        }
        let cell = f64::from(cell as f32);
        let pad = band + cell;
        Lattice {
            // Saturating, because a mesh in the wrong unit saturates the cast
            // at `usize::MAX`, and one more than that would wrap to a count
            // small enough to pass every check below.
            dims: [0, 1, 2]
                .map(|c| (((hi[c] - lo[c] + 2.0 * pad) / cell).ceil() as usize).saturating_add(1)),
            origin: [0, 1, 2].map(|c| f64::from((lo[c] - pad) as f32)),
            cell,
            band,
        }
    }

    /// How many samples the lattice holds.
    pub(super) fn len(&self) -> usize {
        self.dims[0] * self.dims[1] * self.dims[2]
    }

    /// How many samples the lattice holds, when that is a field a bake will
    /// allocate.
    ///
    /// # Errors
    /// `TooLarge` past `cap`, and past what the count itself can hold: a
    /// product that wrapped would size the field to something small and bake
    /// the body into the wrong shape rather than refuse it.
    fn within(&self, cap: usize) -> Result<usize, BakeError> {
        self.dims[0]
            .checked_mul(self.dims[1])
            .and_then(|n| n.checked_mul(self.dims[2]))
            .filter(|n| *n <= cap)
            .ok_or(BakeError::TooLarge(self.dims))
    }

    /// Where one sample sits in the mesh's own frame.
    pub(super) fn point(&self, i: usize, j: usize, k: usize) -> [f64; 3] {
        let n = [i, j, k];
        [0, 1, 2].map(|c| self.origin[c] + n[c] as f64 * self.cell)
    }

    /// One sample's place in the data, in the same order the field is stored
    /// and sampled in: x fastest, z slowest.
    pub(super) fn index(&self, i: usize, j: usize, k: usize) -> usize {
        (k * self.dims[1] + j) * self.dims[0] + i
    }

    /// The range of sample indices along one axis that a span of that axis
    /// reaches, clamped to the lattice.
    pub(super) fn span(&self, axis: usize, lo: f64, hi: f64) -> std::ops::RangeInclusive<usize> {
        let last = (self.dims[axis] - 1) as f64;
        let first = ((lo - self.origin[axis]) / self.cell)
            .floor()
            .clamp(0.0, last);
        let last = ((hi - self.origin[axis]) / self.cell)
            .ceil()
            .clamp(0.0, last);
        first as usize..=last as usize
    }
}

/// Bakes a body mesh into the field the solver collides against.
///
/// Signed distance by Bærentzen–Aanæs: the closest point on the surface, and
/// the sign of its angle-weighted pseudo-normal against the way one has to
/// travel to reach it.
///
/// # Errors
/// If the mesh is empty, carries a position that is not a number, or is not a
/// closed, orientable 2-manifold wound outward — see [`Manifold`] for why
/// that last one is refused rather than baked into a field whose sign would
/// be wrong in places nothing points at.
pub fn sdf(mesh: &BodyMesh) -> Result<SdfGrid, BakeError> {
    mesh_sdf(&mesh.positions, &mesh.indices, CELL, BAND)
}

/// [`sdf`] with the two figures loose, so a test can bake something small
/// enough to check by hand.
fn mesh_sdf(
    positions: &[f32],
    indices: &[u32],
    cell: f64,
    band: f64,
) -> Result<SdfGrid, BakeError> {
    if indices.len() < 3 || positions.len() < 9 {
        return Err(BakeError::Empty);
    }
    if !positions.iter().all(|f| f.is_finite()) {
        return Err(BakeError::NotFinite);
    }
    let topology = inspect(positions, indices);
    if !topology.earns_the_sign() {
        return Err(BakeError::NotClosed(topology));
    }

    let lattice = Lattice::around(positions, cell, band);
    let voxels = lattice.within(MAX_VOXELS)?;
    let pseudo = Pseudo::build(positions, indices);
    let mut data = vec![0.0f32; voxels];
    let mut nearest = vec![f32::INFINITY; voxels];
    band::walk(
        &lattice,
        positions,
        indices,
        &pseudo,
        &mut data,
        &mut nearest,
    );
    fill::saturate(&lattice, &nearest, &mut data);

    Ok(SdfGrid {
        dims: lattice.dims,
        cell: cell as f32,
        origin: lattice.origin.map(|c| c as f32),
        data,
    })
}
