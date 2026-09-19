use super::state::{DistanceConstraints, Seams, State};

mod grid;
mod joined;
#[cfg(test)]
mod tests;
mod touch;

use grid::Grid;
use joined::Joined;

/// Cloth thickness as a fraction of the mesh's own mean edge.
///
/// A thickness in millimetres would mean nothing on its own: it is only ever
/// read against the mesh that has to hold it, and a garment meshed for a
/// trouser has edges twice the length of one meshed for a cuff.
///
/// The ceiling is the closest pair a flat mesh holds without joining: the far
/// corner of a split quad, half a diagonal away, which is 0.71 of an edge on
/// a grid and 0.87 on the roughly equilateral triangles the mesher makes.
/// Above that a panel lying flat starts pushing itself into ripples at rest.
/// The floor is that a discrete pass has to see a layer before it arrives.
/// A third of an edge sits between the two with a factor of two either way.
const THICKNESS_OF_EDGE: f32 = 0.35;

/// Grid cell as a multiple of that same mean edge.
///
/// One edge to a cell leaves a triangle's own box about a cell across, so the
/// box a query walks is two cells on a side and carries a handful of
/// candidates rather than a neighbourhood.
const CELL_OF_EDGE: f32 = 1.0;

/// Cloth against itself: what stops a garment falling through its own folds.
///
/// Off unless a caller builds one. A scene that hands [`super::substep`] no
/// layers runs the passes it has always run, in the order it has always run
/// them, and the goldens are all taken that way.
///
/// It holds the triangles and the sewing it was given, so a mesh swap
/// replaces it rather than editing it; a state of another length is refused
/// rather than indexed.
pub struct Layers {
    tris: Vec<u32>,
    joined: Joined,
    grid: Grid,
    n_verts: usize,
    thickness: f32,
    cell: f32,
    contacts: u64,
}

impl Layers {
    /// Cloth of the thickness this mesh's own edges imply.
    ///
    /// `tris` and `seams` are the whole product's, in the numbering the
    /// combined solver state uses, and `cons` is what the mean edge is read
    /// off. A constraint set with no edges leaves a thickness of zero, which
    /// is a pass that does nothing at all.
    pub fn of(tris: &[u32], cons: &DistanceConstraints, seams: &Seams, n_verts: usize) -> Layers {
        let edge = mean_rest(cons);
        Layers {
            tris: tris.to_vec(),
            joined: Joined::of(tris, seams, n_verts),
            grid: Grid::new(),
            n_verts,
            thickness: edge * THICKNESS_OF_EDGE,
            cell: (edge * CELL_OF_EDGE).max(f32::MIN_POSITIVE),
            contacts: 0,
        }
    }

    /// How far apart two layers are held, in metres.
    pub fn thickness(&self) -> f32 {
        self.thickness
    }

    /// Vertex-to-triangle contacts parted since this was built.
    ///
    /// The sensor the thickness is chosen against: a panel lying flat has to
    /// report none at all, because every pair close enough to count there is
    /// one the mesh itself holds together.
    pub fn contacts(&self) -> u64 {
        self.contacts
    }

    /// Parts every layer that has come within the thickness of another.
    ///
    /// Sequential over the triangles in their stored order, over the cells of
    /// the grid in one fixed scan, and over each cell's vertices in ascending
    /// order — a later pair sees what an earlier one moved, and sees it the
    /// same way on every machine.
    ///
    /// `dt` is the substep being solved, in seconds: how far one contact may
    /// correct is a speed, and it is only a distance against that.
    pub(super) fn separate(&mut self, state: &mut State, dt: f32) {
        // A swap replaces the triangles and the state together. Colliding one
        // against the other is a wrong drape rather than a slow one, so a
        // mismatch is refused here instead of indexed past the end.
        if self.thickness <= 0.0 || state.len() != self.n_verts {
            return;
        }
        let Layers {
            tris,
            joined,
            grid,
            thickness,
            cell,
            ..
        } = self;
        grid.build(state, *cell);
        let reach = touch::reach(dt);

        let mut hits = 0u64;
        for t in tris.as_chunks::<3>().0 {
            let corner = [t[0] as usize, t[1] as usize, t[2] as usize];
            let (lo, hi) = box_of(state, corner, *thickness);
            let span = grid.range(lo, hi);
            for k in span[2][0]..=span[2][1] {
                for j in span[1][0]..=span[1][1] {
                    for i in span[0][0]..=span[0][1] {
                        for &v in grid.at(i, j, k) {
                            if t.contains(&v) || !inside(state, v as usize, lo, hi) {
                                continue;
                            }
                            if t.iter().any(|&w| joined.holds(v, w)) {
                                continue;
                            }
                            if touch::part(state, v as usize, corner, *thickness, reach) {
                                hits += 1;
                            }
                        }
                    }
                }
            }
        }
        self.contacts += hits;
    }
}

/// The box a triangle's thickness reaches, which is what the grid is asked
/// for and what rejects a candidate before its closest point is solved.
fn box_of(state: &State, t: [usize; 3], thickness: f32) -> ([f32; 3], [f32; 3]) {
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for &v in &t {
        let p = [state.px[v], state.py[v], state.pz[v]];
        for c in 0..3 {
            lo[c] = lo[c].min(p[c]);
            hi[c] = hi[c].max(p[c]);
        }
    }
    for c in 0..3 {
        lo[c] -= thickness;
        hi[c] += thickness;
    }
    (lo, hi)
}

/// Whether a vertex falls inside that box.
fn inside(state: &State, v: usize, lo: [f32; 3], hi: [f32; 3]) -> bool {
    let p = [state.px[v], state.py[v], state.pz[v]];
    (0..3).all(|c| p[c] >= lo[c] && p[c] <= hi[c])
}

/// Mean rest length over the stretch constraints, in metres.
///
/// Summed in the constraints' own order and on one thread, so the thickness a
/// mesh derives is the same number on every run and every machine — the same
/// reason nothing else here is reduced in parallel.
fn mean_rest(cons: &DistanceConstraints) -> f32 {
    if cons.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0f32;
    for &rest in &cons.rest {
        sum += rest;
    }
    sum / cons.len() as f32
}
