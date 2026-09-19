use super::pierce::Point;

/// A triangle's box: its lowest and its highest corner.
pub(super) type Span = (Point, Point);

/// How many of the average triangle's longest side a cell is across.
///
/// Two puts a handful of triangles in a cell whatever unit the mesh is in.
/// It is a starting point and not a tuned number: on the Anny body anything
/// from a half to four costs the same to within the noise of the clock,
/// because what is paid for is the pairs whose boxes really do touch, and
/// those are the same pairs under any cell.
const SIDES_PER_CELL: f64 = 2.0;

/// The most cells a grid is given for each triangle filed in it, so that the
/// table of cells stays the size of the mesh and not of the air around it.
const CELLS_PER_TRIANGLE: usize = 4;

/// The most cells the average triangle may be filed under.
///
/// A mesh that mixes a few triangles the size of the whole box with many
/// small ones would file each large one under every cell there is. Past
/// this the cells are doubled instead, and the price moves to more pairs a
/// cell, which a box test throws out one comparison at a time.
const FILINGS_PER_TRIANGLE: usize = 32;

/// Where the cells are: the corner they start from, their size, their count.
struct Frame {
    lo: Point,
    cell: f64,
    dims: [usize; 3],
}

impl Frame {
    /// The cell a coordinate falls in along one axis, clamped to the grid.
    ///
    /// Nothing filed here is below `lo`, so the cast only ever floors.
    fn coord(&self, axis: usize, x: f64) -> usize {
        (((x - self.lo[axis]) / self.cell) as usize).min(self.dims[axis] - 1)
    }

    /// The lowest and the highest cell a box reaches.
    fn reach(&self, span: &Span) -> ([usize; 3], [usize; 3]) {
        (
            [0, 1, 2].map(|c| self.coord(c, span.0[c])),
            [0, 1, 2].map(|c| self.coord(c, span.1[c])),
        )
    }

    /// One cell's place in the scan: x fastest, z slowest.
    fn index(&self, at: [usize; 3]) -> usize {
        (at[2] * self.dims[1] + at[1]) * self.dims[0] + at[0]
    }

    /// Whether filing `boxes` under cells this size stays inside both caps.
    fn affordable(&self, boxes: &[Span]) -> bool {
        let cells = self.dims[0]
            .checked_mul(self.dims[1])
            .and_then(|n| n.checked_mul(self.dims[2]));
        if cells.is_none_or(|n| n > CELLS_PER_TRIANGLE * boxes.len() + 64) {
            return false;
        }
        let mut filings = 0;
        for span in boxes {
            let (a, z) = self.reach(span);
            filings += (0..3).map(|c| z[c] - a[c] + 1).product::<usize>();
            if filings > FILINGS_PER_TRIANGLE * boxes.len() {
                return false;
            }
        }
        true
    }
}

/// Triangle boxes filed under the cells of a uniform grid.
///
/// Counted, then filled, in triangle order, so a cell lists its triangles by
/// rising index and the whole structure is two flat arrays: nothing here
/// hashes, and nothing is ordered by anything but the mesh itself.
pub(super) struct Grid {
    frame: Frame,
    /// The lowest cell each triangle's box reaches.
    first: Vec<[usize; 3]>,
    /// Cell `c` lists `filed[starts[c]..starts[c + 1]]`.
    starts: Vec<usize>,
    filed: Vec<u32>,
}

impl Grid {
    /// Files every box under the cells it reaches.
    ///
    /// The cell is sized from the triangles rather than written down, so a
    /// mesh in any unit gets a few triangles a cell; it is then doubled until
    /// the grid is affordable, which a single cell always is.
    pub(super) fn over(boxes: &[Span]) -> Grid {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        let mut sides = 0.0;
        for (a, z) in boxes {
            for c in 0..3 {
                lo[c] = lo[c].min(a[c]);
                hi[c] = hi[c].max(z[c]);
            }
            sides += (0..3).map(|c| z[c] - a[c]).fold(0.0, f64::max);
        }
        let mean = sides / boxes.len().max(1) as f64;
        let mut cell = if mean > 0.0 {
            SIDES_PER_CELL * mean
        } else {
            1.0
        };
        let frame = loop {
            let dims = [0, 1, 2].map(|c| (((hi[c] - lo[c]) / cell) as usize).saturating_add(1));
            let frame = Frame { lo, cell, dims };
            if frame.affordable(boxes) {
                break frame;
            }
            cell *= 2.0;
        };
        Grid::file(frame, boxes)
    }

    /// Counts what each cell will hold, then writes each list in place.
    fn file(frame: Frame, boxes: &[Span]) -> Grid {
        let cells = frame.dims[0] * frame.dims[1] * frame.dims[2];
        let mut starts = vec![0usize; cells + 1];
        let mut first = Vec::with_capacity(boxes.len());
        for span in boxes {
            let (a, z) = frame.reach(span);
            each_cell(a, z, |at| starts[frame.index(at) + 1] += 1);
            first.push(a);
        }
        for c in 0..cells {
            starts[c + 1] += starts[c];
        }
        let mut next = starts.clone();
        let mut filed = vec![0u32; starts[cells]];
        for (t, span) in boxes.iter().enumerate() {
            let (a, z) = frame.reach(span);
            each_cell(a, z, |at| {
                let slot = &mut next[frame.index(at)];
                filed[*slot] = t as u32;
                *slot += 1;
            });
        }
        Grid {
            frame,
            first,
            starts,
            filed,
        }
    }

    /// Every pair of triangles whose boxes touch, lower index first, each
    /// exactly once and in the same order on every run.
    ///
    /// Two boxes that touch are filed together under every cell they share,
    /// and a pair must not be answered once a cell. It is answered in the one
    /// cell that holds the lowest corner of what the two boxes have in
    /// common, which needs no record of the pairs already seen.
    pub(super) fn pairs(&self, boxes: &[Span], mut visit: impl FnMut(u32, u32)) {
        for cell in 0..self.starts.len() - 1 {
            let here = &self.filed[self.starts[cell]..self.starts[cell + 1]];
            for (n, &a) in here.iter().enumerate() {
                for &b in &here[n + 1..] {
                    if touch(&boxes[a as usize], &boxes[b as usize]) && self.home(a, b) == cell {
                        visit(a, b);
                    }
                }
            }
        }
    }

    /// The one cell a pair is answered in.
    fn home(&self, a: u32, b: u32) -> usize {
        let (a, b) = (self.first[a as usize], self.first[b as usize]);
        self.frame.index([0, 1, 2].map(|c| a[c].max(b[c])))
    }
}

/// Whether two boxes share any point.
fn touch(a: &Span, b: &Span) -> bool {
    (0..3).all(|c| a.0[c] <= b.1[c] && b.0[c] <= a.1[c])
}

/// Walks the cells from `a` to `z`, both included.
fn each_cell(a: [usize; 3], z: [usize; 3], mut visit: impl FnMut([usize; 3])) {
    for k in a[2]..=z[2] {
        for j in a[1]..=z[1] {
            for i in a[0]..=z[0] {
                visit([i, j, k]);
            }
        }
    }
}
