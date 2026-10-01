use toile_doc::{EdgeRange, Seam, SeamKind};

use super::Draft;

/// What the two sides of a seam measure, in centimetres.
///
/// One judge for every panel and every report that renders a verdict about a
/// seam: the table under the drape, the inspector over the drawing and the
/// headless door onto a pattern read the same lengths against the same
/// tolerance, so they can never disagree about one seam.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lengths {
    /// Side A, walked head to tail along its contour.
    pub a: f64,
    /// Side B, walked the same way.
    pub b: f64,
}

impl Lengths {
    /// Both sides measured, when the draft can answer for all four anchors.
    pub fn of(draft: &Draft, seam: &Seam) -> Option<Lengths> {
        let (a, b) = side_cm(draft, &seam.a).zip(side_cm(draft, &seam.b))?;
        Some(Lengths { a, b })
    }

    /// How much longer side A is than side B; negative when B is the longer.
    pub fn delta(self) -> f64 {
        self.a - self.b
    }

    /// How far the mismatch is from the one the seam is meant to carry, in
    /// centimetres, with the sign dropped.
    ///
    /// What is judged, and what a report shows beside the tolerance: an eased
    /// seam asks for a difference, so the number that matters is never the
    /// difference itself but how far it is from the one asked for.
    pub fn excess(self, seam: &Seam) -> f64 {
        (self.delta().abs() - seam.expected_cm()).abs()
    }

    /// Whether the two lengths agree, by the seam's own rule.
    ///
    /// `None` for a gathered seam: it is judged on a ratio of a long side to a
    /// short one, and the document does not say which of the two is which. A
    /// mark drawn from a guess about that is exactly the verdict a panel must
    /// not render.
    pub fn meets(self, draft: &Draft, seam: &Seam) -> Option<bool> {
        if matches!(seam.kind, SeamKind::Gathered { .. }) {
            return None;
        }
        Some(self.excess(seam) <= tolerance_cm(draft, seam))
    }
}

/// The centimetres of mismatch a seam carries before it complains: its own
/// when it has one, the document's variable otherwise.
pub fn tolerance_cm(draft: &Draft, seam: &Seam) -> f64 {
    seam.tolerance
        .or_else(|| draft.env().value(seam.tolerance_variable()))
        .unwrap_or(Seam::DEFAULT_TOLERANCE_CM)
}

/// What one side measures along its piece's flattened contour, in centimetres.
///
/// The walk is the one the contour runs, head to tail, over the same fractions
/// the solver pairs the two sides on; a side that passes the closure is as long
/// as that walk and never as short as the gap it leaves.
pub fn side_cm(draft: &Draft, range: &EdgeRange) -> Option<f64> {
    let piece = range.piece()?;
    let head = draft.anchor_fraction(&range.head)?;
    let tail = draft.anchor_fraction(&range.tail)?;
    Some((tail - head).rem_euclid(1.0) * draft.perimeter_cm(piece))
}

#[cfg(test)]
mod tests;
