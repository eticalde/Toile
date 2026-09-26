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
    /// One stretch on the cloth: which piece it lands on, where that piece's
    /// edges begin, and the boundary vertices it runs over.
    ///
    /// `None` for a stretch the cloth cannot answer for — a piece too partial
    /// to mesh, an anchor on a node its piece no longer runs through, or a
    /// stretch with no length. Left out of the solve rather than guessed
    /// at: a stretch read onto the wrong vertices would hold the garment by
    /// somewhere nobody asked for.
    ///
    /// Read afresh rather than remembered, for the reason the sewing is: a
    /// shape edit moves an anchor's fraction along its own piece and a
    /// rebuild hands that piece a whole new set of vertices, so a reading
    /// is only ever true of the meshes it was taken against.
    pub(super) fn run_of(
        &self,
        draft: &Draft,
        pipes: &[&ShapePipeline],
        bases: &[usize],
        at: EdgeRange,
    ) -> Option<Run> {
        let piece = self.index_of(at.piece()?)?;
        let head = draft.anchor_fraction(&at.head)?;
        let tail = draft.anchor_fraction(&at.tail)?;
        // The walk the way the contour runs, as a seam's side is measured, so
        // a stretch that passes the closure is as long as the walk.
        let span = (tail - head).rem_euclid(1.0);
        if span <= f64::EPSILON {
            return None;
        }
        let verts = pipes[piece].boundary_run((head, span));
        (verts.len() >= 2).then(|| Run {
            at: piece,
            base: bases[piece],
            verts,
        })
    }
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
