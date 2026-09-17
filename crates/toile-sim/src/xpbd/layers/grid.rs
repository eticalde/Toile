use crate::xpbd::state::State;

/// Most cells the grid may hold per particle before it coarsens itself.
///
/// Cloth is a surface, so the box around it is mostly air and the cost of that
/// air is memory rather than query time. The cap is what stops a garment
/// stretched across a room from asking for a grid that will not fit.
const CELLS_PER_PARTICLE: usize = 32;

/// Most cells along one axis, so one stray coordinate cannot overflow the
/// product of the three counts.
const LONGEST: usize = 1024;

/// A uniform grid over the particles, rebuilt every substep.
///
/// Self-collision pairs change every substep, so unlike the stretch
/// constraints they cannot be coloured once and reused. What is reused is the
/// memory: the buffers are resized, never reallocated.
///
/// Filled by a counting sort, so a cell holds its vertices in ascending order,
/// and walked in one fixed scan order: no hash, no order a thread pool could
/// change, nothing sorted on a key with ties. The same positions therefore
/// give the same drape on every machine and every run.
///
/// That is reproducibility, and it is all it is. The pass is Gauss-Seidel, so
/// a later contact sees what an earlier one moved: the scan order is part of
/// the answer and not merely of the bookkeeping. Walking it backwards is a
/// different drape — measured, nearly twice the contacts and nearly three
/// times the crossings left behind. The order chosen here is a convention that
/// happens to be the better of the two, not a physical result.
pub(super) struct Grid {
    lo: [f32; 3],
    cell: f32,
    dims: [usize; 3],
    /// Where each cell's run begins, with a final sentinel.
    start: Vec<u32>,
    /// Write cursor per cell while the sort scatters.
    fill: Vec<u32>,
    verts: Vec<u32>,
}

impl Grid {
    /// An empty grid, for a [`super::Layers`] that has not stepped yet.
    pub(super) fn new() -> Grid {
        Grid {
            lo: [0.0; 3],
            cell: 1.0,
            dims: [1; 3],
            start: Vec::new(),
            fill: Vec::new(),
            verts: Vec::new(),
        }
    }

    /// Bins every particle, coarsening the cell until the grid fits its cap.
    pub(super) fn build(&mut self, state: &State, cell: f32) {
        let n = state.len();
        let (lo, hi) = extent(state);
        self.lo = lo;
        self.cell = cell.max(f32::MIN_POSITIVE);
        self.dims = counts(lo, hi, self.cell);
        let cap = n.max(1).saturating_mul(CELLS_PER_PARTICLE);
        for _ in 0..24 {
            if self.dims[0] * self.dims[1] * self.dims[2] <= cap {
                break;
            }
            self.cell *= 2.0;
            self.dims = counts(lo, hi, self.cell);
        }

        let cells = self.dims[0] * self.dims[1] * self.dims[2];
        self.start.clear();
        self.start.resize(cells + 1, 0);
        for i in 0..n {
            let c = self.index_of(state, i);
            self.start[c + 1] += 1;
        }
        for c in 0..cells {
            self.start[c + 1] += self.start[c];
        }
        self.fill.clear();
        self.fill.extend_from_slice(&self.start[..cells]);
        self.verts.clear();
        self.verts.resize(n, 0);
        for i in 0..n {
            let c = self.index_of(state, i);
            self.verts[self.fill[c] as usize] = i as u32;
            self.fill[c] += 1;
        }
    }

    /// The cells a box covers, as an inclusive first and last per axis.
    pub(super) fn range(&self, lo: [f32; 3], hi: [f32; 3]) -> [[usize; 2]; 3] {
        let (a, b) = (self.axis_of(lo), self.axis_of(hi));
        [[a[0], b[0]], [a[1], b[1]], [a[2], b[2]]]
    }

    /// The vertices in one cell, ascending.
    pub(super) fn at(&self, i: usize, j: usize, k: usize) -> &[u32] {
        let c = (k * self.dims[1] + j) * self.dims[0] + i;
        &self.verts[self.start[c] as usize..self.start[c + 1] as usize]
    }

    fn index_of(&self, state: &State, i: usize) -> usize {
        let at = self.axis_of([state.px[i], state.py[i], state.pz[i]]);
        (at[2] * self.dims[1] + at[1]) * self.dims[0] + at[0]
    }

    /// Which cell a point falls in, clamped to the grid on every axis.
    ///
    /// A coordinate that is not a number compares false against zero and lands
    /// in the first cell, which keeps the index in bounds. Cloth carrying one
    /// has already lost its drape; what matters here is that it does not also
    /// lose the process.
    fn axis_of(&self, p: [f32; 3]) -> [usize; 3] {
        let mut at = [0usize; 3];
        for c in 0..3 {
            let f = (p[c] - self.lo[c]) / self.cell;
            at[c] = if f > 0.0 {
                (f as usize).min(self.dims[c] - 1)
            } else {
                0
            };
        }
        at
    }
}

/// Cells along each axis for a box of this size.
fn counts(lo: [f32; 3], hi: [f32; 3], cell: f32) -> [usize; 3] {
    let mut dims = [1usize; 3];
    for c in 0..3 {
        let span = (hi[c] - lo[c]) / cell;
        if span.is_finite() && span > 0.0 {
            dims[c] = (span as usize + 1).min(LONGEST);
        }
    }
    dims
}

/// The lowest and highest corner of the cloth.
fn extent(state: &State) -> ([f32; 3], [f32; 3]) {
    let mut lo = [0.0f32; 3];
    let mut hi = [0.0f32; 3];
    for (c, axis) in [&state.px, &state.py, &state.pz].into_iter().enumerate() {
        let low = axis.iter().copied().fold(f32::INFINITY, f32::min);
        let high = axis.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        if low.is_finite() && high.is_finite() {
            lo[c] = low;
            hi[c] = high;
        }
    }
    (lo, hi)
}
