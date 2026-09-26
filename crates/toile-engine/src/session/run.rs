use toile_doc::EdgeRange;

use super::{Session, ShapePipeline};
use crate::draft::Draft;

/// One stretch of contour read onto one piece's mesh: the boundary vertices it
/// runs over and where that piece's edges open in the constraint set.
pub(super) struct Run {
    pub(super) at: usize,
    pub(super) base: usize,
    pub(super) verts: Vec<u32>,
}

impl Session {
    /// Every stretch on the cloth one range of contour names: the piece each
    /// lands on, where its edges begin, and the vertices the stretch runs over.
    ///
    /// One stretch for a piece drawn whole. Two for a piece drawn against a
    /// fold: the cloth is the drawing and its reflection, so a range written
    /// once is on the cut piece twice, and reading the drawn half alone would
    /// hold half of the cloth and say nothing of the other half.
    ///
    /// Empty for a range the cloth cannot answer for, and for one only one half
    /// of a folded piece has the cloth to carry. Left out of the solve rather
    /// than guessed at: a range read onto the wrong vertices would hold the
    /// garment by somewhere nobody asked for.
    ///
    /// Read afresh rather than remembered, for the reason the sewing is: a
    /// shape edit moves an anchor's fraction along its own piece and a rebuild
    /// hands that piece a whole new set of vertices, so a reading is only ever
    /// true of the meshes it was taken against.
    pub(super) fn runs_of(
        &self,
        draft: &Draft,
        pipes: &[&ShapePipeline],
        bases: &[usize],
        at: EdgeRange,
    ) -> Vec<Run> {
        let Some(key) = at.piece() else {
            return Vec::new();
        };
        let (Some(piece), Some(drawn)) = (self.index_of(key), fractions(draft, &at)) else {
            return Vec::new();
        };
        let mirror = draft.cloth(key).and_then(|cloth| cloth.mirror_run(drawn));
        let mut runs = Vec::new();
        for stretch in [Some(drawn), mirror].into_iter().flatten() {
            let verts = pipes[piece].boundary_run(stretch);
            // Both halves of a folded piece or neither of them. The mesh lays
            // its own vertices, so a stretch this short can come to two of
            // them on one half and one on the other, and keeping the half that
            // came out holds the cloth on one side of the crease alone: half
            // the cloth when that half is the drawn one, and a place nobody
            // drew when it is the mirror.
            if verts.len() < 2 {
                return Vec::new();
            }
            runs.push(Run {
                at: piece,
                base: bases[piece],
                verts,
            });
        }
        runs
    }
}

/// Where a range of contour opens on its piece's cloth and how far it runs,
/// both as fractions of that cloth's perimeter.
///
/// `None` when either end is a place the cloth has no boundary at — a node the
/// piece no longer runs through, or one inside the crease of a fold — and for
/// a range with no length.
fn fractions(draft: &Draft, at: &EdgeRange) -> Option<(f64, f64)> {
    let head = draft.anchor_fraction(&at.head)?;
    let tail = draft.anchor_fraction(&at.tail)?;
    // The walk the way the contour runs, as a seam's side is measured, so
    // a stretch that passes the closure is as long as the walk.
    let span = (tail - head).rem_euclid(1.0);
    (span > f64::EPSILON).then_some((head, span))
}

/// Where each piece's edges begin in the product's combined constraints.
///
/// The constraints are concatenated in the order the pieces stand, so a
/// piece's edges begin past every edge of every piece before it — the same
/// arithmetic `couture::offsets` does over vertices, counted over edges
/// because that is what an elastic addresses.
pub(super) fn edge_bases(pipes: &[&ShapePipeline]) -> Vec<usize> {
    let mut at = 0;
    let mut bases = Vec::with_capacity(pipes.len());
    for pipe in pipes {
        bases.push(at);
        at += pipe.edges.len();
    }
    bases
}
