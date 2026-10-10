use super::{Panel, Round};

/// Where a declared heading fixes one line of cloth on the surface.
///
/// What it replaces is the document's storage order. With nothing pinned the
/// turn opens at the middle of whichever panel the walk reaches first, so the
/// piece a file happens to hold first puts its own middle at the centre front:
/// measured on one three-panel blouse, 44.6° round with a front stored first
/// and 180.0° — back to front — with the back.
///
/// One line and not a line per piece, which is what makes it enough: every
/// other point is placed by the cloth walked to reach it, so pinning one fixes
/// the whole strip however the pieces are stored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pin {
    /// The piece the pinned line belongs to.
    pub piece: usize,
    /// Its pattern abscissa, in metres.
    pub at: f64,
    /// The turn it is pinned at, in radians from the centre front, growing
    /// toward the left of whoever wears the garment.
    ///
    /// Which is `+x`, measured rather than assumed. The body is laid out facing
    /// `+z`: its soles reach 16 cm forward of the ankle and 12 cm back, so the
    /// forefoot names the front. And for a person facing `+z` with `+y` up,
    /// their left hand is at `+x`. The layout puts a turn of zero at `+z` and a
    /// quarter lap at `+x`, so a turn that grows is a turn going leftward.
    pub turn: f64,
    /// `1.0` when that piece's rising abscissa goes toward the wearer's left,
    /// `-1.0` when it goes to their right.
    ///
    /// Half of what a heading says, and the half an angle cannot carry: a
    /// pinned point fixes where the cloth starts and leaves the strip free to
    /// go round either way. Measured, that is how a buttoned blouse comes out
    /// mirrored with 3 of its 3 panels running against the turn.
    pub leftward: f64,
}

impl Round {
    /// Which way round the body one pattern abscissa of a piece faces at an
    /// ordinate: degrees from the centre front, growing toward the wearer's
    /// left, in (-180, 180]. `None` for a piece the strip does not carry.
    ///
    /// Through [`Round::at`] and not by arithmetic of its own, which is the
    /// whole reason it is written this way: a reading of where a garment faces
    /// cannot be derived from the placement, because it *is* the placement. A
    /// second formula that agrees today is a second formula to keep in step for
    /// ever, and the day it drifts it says the product faces where it does not.
    pub fn facing(&self, piece: usize, abscissa: f64, y: f64) -> Option<f64> {
        let (turn, _) = self.at(piece, [abscissa, y])?;
        Some(lap(turn.to_degrees()))
    }
}

/// How much of one panel's cloth the walk has crossed to reach an abscissa of
/// it, given the arc `lo..hi` that panel is allotted at this ordinate.
///
/// The one place the walk's own direction is read, so the pinned line and every
/// placed vertex are measured into a panel the same way. Measured apart, a pin
/// on a panel the walk crosses backwards opens the turn at the cloth beyond the
/// point instead of the cloth before it.
pub(super) fn into(panel: &Panel, abscissa: f64, lo: f64, hi: f64) -> f64 {
    if panel.sense > 0.0 {
        abscissa - lo
    } else {
        hi - abscissa
    }
}

/// Which way the walk crosses a piece: `1.0` with its rising abscissa, `-1.0`
/// against it, and `1.0` for a piece it does not carry at all.
///
/// The fallback is never reached from [`Round::over`], which drops a pin whose
/// piece is not in the walk before it asks. It is here so that the answer for a
/// piece nobody walks is the placement every release had before a heading could
/// be written, rather than a panel chosen by position.
pub(super) fn sense_of(panels: &[Panel], piece: usize) -> f64 {
    panels
        .iter()
        .find(|panel| panel.piece == piece)
        .map_or(1.0, |panel| panel.sense)
}

/// An angle folded into the one lap a heading is written in: (-180, 180].
///
/// Half-open, and 180 is the end that is included, because 180 is the centre
/// back — one of the four places of the trade, and the one that sits on the
/// seam between the two spellings. A reading that handed back −180 for it would
/// not match the turn the person who declared it typed.
fn lap(degrees: f64) -> f64 {
    let round = degrees.rem_euclid(360.0);
    if round > 180.0 { round - 360.0 } else { round }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every angle reads back in the lap a heading is declared in, and the
    /// centre back has the one spelling the document gives it.
    #[test]
    fn the_lap_a_reading_comes_back_in_is_the_lap_a_heading_is_written_in() {
        for (turn, folded) in [
            (0.0, 0.0),
            (90.0, 90.0),
            (180.0, 180.0),
            (-90.0, -90.0),
            (-180.0, 180.0),
            (181.0, -179.0),
            (360.0, 0.0),
            (450.0, 90.0),
            (-270.0, 90.0),
        ] {
            assert!((lap(turn) - folded).abs() < 1.0e-12, "{turn} read {folded}");
        }
    }
}
