use thiserror::Error;
use toile_doc::{EdgeAnchor, Seam, SeamOrientation};

use crate::couture::{ShapePipeline, pair_seam};
use crate::draft::Draft;

/// What stops a document seam from being paired for sewing.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SeamFault {
    /// An anchor names a node its piece does not run through.
    #[error("a seam anchor sits on a node its piece does not run through")]
    Unanchored,
    /// Head and tail of one side land on the same spot.
    #[error("a seam side has no length between its anchors")]
    EmptyRange,
}

/// Pairs the two sides of a document seam into solver vertex pairs.
///
/// This is where an anchored seam becomes the pair of fraction ranges
/// [`pair_seam`] walks. Fractions are read from the draft at call time, so an
/// edit that changes the perimeter elsewhere cannot slide the seam along the
/// cloth; the pair count comes from the finer side's boundary spacing, and
/// `Opposed` hands side `b` over tail to head. Each side states where its own
/// piece begins in the combined solver state — see [`pair_seam`].
///
/// # Errors
/// `Unanchored` when an anchor names a node its piece does not run through;
/// `EmptyRange` when a side's head and tail land on the same spot.
pub fn pair_seam_anchored(
    draft: &Draft,
    seam: &Seam,
    a: &ShapePipeline,
    a_offset: u32,
    b: &ShapePipeline,
    b_offset: u32,
) -> Result<(Vec<u32>, Vec<u32>), SeamFault> {
    let read = |at: &EdgeAnchor| draft.anchor_fraction(at).ok_or(SeamFault::Unanchored);
    let (head_a, tail_a) = (read(&seam.a.head)?, read(&seam.a.tail)?);
    let (head_b, tail_b) = (read(&seam.b.head)?, read(&seam.b.tail)?);

    // A span is measured the way the contour runs, so a side that passes the
    // closure is as long as the walk and never as short as the gap it leaves.
    let span = |head: f64, tail: f64| (tail - head).rem_euclid(1.0);
    let (span_a, span_b) = (span(head_a, tail_a), span(head_b, tail_b));
    if span_a <= f64::EPSILON || span_b <= f64::EPSILON {
        return Err(SeamFault::EmptyRange);
    }

    let per_side = |s: f64, boundary: usize| (s * boundary as f64).ceil() as usize;
    let pairs = per_side(span_a, a.n_boundary())
        .max(per_side(span_b, b.n_boundary()))
        .max(2);

    // The span goes over as itself. Handing `pair_seam` the far end instead
    // would make it subtract the near one back out, and that round trip does
    // not return the span it was given.
    let run_b = match seam.orientation {
        SeamOrientation::Aligned => (head_b, span_b),
        SeamOrientation::Opposed => (tail_b, -span_b),
    };
    Ok(pair_seam(
        a,
        (head_a, span_a),
        a_offset,
        b,
        run_b,
        b_offset,
        pairs,
    ))
}
