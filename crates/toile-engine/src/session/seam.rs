use thiserror::Error;
use toile_doc::{EdgeAnchor, Seam, SeamOrientation};

use crate::couture::ShapePipeline;
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
/// Fractions are read from the draft at call time, so an edit that changes
/// the perimeter elsewhere cannot slide the seam along the cloth. The pair
/// count comes from the finer side's boundary spacing; `Opposed` walks side
/// `b` tail to head. Indices from `b` are offset by `b_offset` into the
/// combined solver state. Fewer pairs come back where the nearest boundary
/// vertex repeats, as on runs sampled more finely than the mesh.
///
/// # Errors
/// `Unanchored` when an anchor names a node its piece does not run through;
/// `EmptyRange` when a side's head and tail land on the same spot.
pub fn pair_seam_anchored(
    draft: &Draft,
    seam: &Seam,
    a: &ShapePipeline,
    b: &ShapePipeline,
    b_offset: u32,
) -> Result<(Vec<u32>, Vec<u32>), SeamFault> {
    let read = |at: &EdgeAnchor| draft.anchor_fraction(at).ok_or(SeamFault::Unanchored);
    let (head_a, tail_a) = (read(&seam.a.head)?, read(&seam.a.tail)?);
    let (head_b, tail_b) = (read(&seam.b.head)?, read(&seam.b.tail)?);

    let span = |head: f64, tail: f64| (tail - head).rem_euclid(1.0);
    let (span_a, span_b) = (span(head_a, tail_a), span(head_b, tail_b));
    if span_a <= f64::EPSILON || span_b <= f64::EPSILON {
        return Err(SeamFault::EmptyRange);
    }

    let per_side = |s: f64, boundary: usize| (s * boundary as f64).ceil() as usize;
    let pairs = per_side(span_a, a.n_boundary())
        .max(per_side(span_b, b.n_boundary()))
        .max(2);

    let mut va = Vec::with_capacity(pairs);
    let mut vb = Vec::with_capacity(pairs);
    for k in 0..pairs {
        let u = k as f64 / (pairs - 1) as f64;
        let fa = (head_a + span_a * u).rem_euclid(1.0);
        let fb = match seam.orientation {
            SeamOrientation::Aligned => (head_b + span_b * u).rem_euclid(1.0),
            SeamOrientation::Opposed => (tail_b - span_b * u).rem_euclid(1.0),
        };
        let (pa, pb) = (
            a.boundary_vertex_near(fa),
            b.boundary_vertex_near(fb) + b_offset,
        );
        if va.last() == Some(&pa) || vb.last() == Some(&pb) {
            continue;
        }
        va.push(pa);
        vb.push(pb);
    }
    Ok((va, vb))
}
