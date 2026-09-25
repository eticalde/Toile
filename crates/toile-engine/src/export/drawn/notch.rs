use toile_geom::validate::signed_area;

use super::{Run, anchor};
use crate::draft::{Draft, Notch, NotchCount, PieceKey};

/// How far a notch's mark reaches into the cloth, in centimetres.
///
/// Inward, and not outward: Toile draws the line the scissors follow, so a mark
/// outside it would sit on the paper a person cuts away, and on a tiled sheet
/// it would sit past the edge the page is held to. Half a centimetre is read
/// across a cutting table, and it is drawn at the weight of everything a cutter
/// does not cut on, so the mark stays a mark and never reads as a second cut
/// line.
const DEEP: f64 = 0.5;

/// How far apart the marks of a double or a triple notch sit, in centimetres.
///
/// They straddle the place the notch names rather than starting at it, so a
/// single, a double and a triple notch all name the same place and only the
/// count differs — which is the whole of what a sewer reads off them.
const APART: f64 = 0.3;

/// Every notch on a piece, each as the marks it is drawn with.
pub(super) fn runs(draft: &Draft, piece: PieceKey) -> Vec<Vec<Run>> {
    draft
        .doc()
        .notches
        .iter()
        .filter(|(_, held)| held.at.piece == piece)
        .filter_map(|(_, held)| marks(draft, held))
        .collect()
}

/// The marks one notch is drawn with, and nothing at all when its place is not
/// on the edge of the cloth.
///
/// The marks of a double or a triple notch are set off along the tangent rather
/// than walked along the flattening: three millimetres away is not where a
/// corner of the contour needs a decision, and a walk would have to make one.
fn marks(draft: &Draft, held: &Notch) -> Option<Vec<Run>> {
    // A place on the crease of a fold is inside the cloth, where there is no
    // edge to notch. It is the same answer the draft gives a seam sent there.
    draft.anchor_fraction(&held.at)?;
    let place = anchor::on(draft, &held.at)?;
    let along = place.along?;
    let inward = inward(draft.flat_cm(held.at.piece), along);
    Some(
        offsets(held.count)
            .iter()
            .map(|&offset| {
                let from = [0, 1].map(|axis| place.at[axis] + along[axis] * offset);
                let to = [0, 1].map(|axis| from[axis] + inward[axis] * DEEP);
                Run::mark(vec![from, to])
            })
            .collect(),
    )
}

/// How far along the contour each mark of a notch sits from the place the notch
/// names.
fn offsets(count: NotchCount) -> &'static [f64] {
    match count {
        NotchCount::Single => &[0.0],
        NotchCount::Double => &[-APART / 2.0, APART / 2.0],
        NotchCount::Triple => &[-APART, 0.0, APART],
    }
}

/// Which way the cloth lies from a contour that runs `along`, as a unit vector.
///
/// Read off the resolved outline and not off the winding the document declares:
/// the declaration is what the drafter meant, the outline is what the scissors
/// will follow, and a formula that turned a piece inside out would put every
/// notch of it in the air.
fn inward(outline: &[[f64; 2]], along: [f64; 2]) -> [f64; 2] {
    let turn = if signed_area(outline) > 0.0 {
        1.0
    } else {
        -1.0
    };
    [-along[1] * turn, along[0] * turn]
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "a quarter turn of an axis of the grid is exact in a double"
    )]

    use super::*;

    /// A ten-centimetre square, drawn clockwise on the page with y downward.
    const SQUARE: [[f64; 2]; 4] = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];

    /// The cloth is on the inside whichever way round the outline was drawn,
    /// which is the one thing a mark cut into it depends on.
    #[test]
    fn a_mark_turns_towards_the_cloth_whichever_way_the_outline_runs() {
        let clockwise = SQUARE;
        let widdershins: Vec<[f64; 2]> = SQUARE.iter().rev().copied().collect();
        // The top side of the square runs towards +x drawn one way round and
        // towards -x drawn the other, and the cloth is below it either way.
        assert_eq!(inward(&clockwise, [1.0, 0.0]), [0.0, 1.0]);
        assert_eq!(inward(&widdershins, [-1.0, 0.0]), [0.0, 1.0]);
    }

    /// A double notch straddles the place it names, so that all three counts
    /// name one place.
    #[test]
    fn every_count_of_notch_is_centred_on_the_place_it_names() {
        for count in [NotchCount::Single, NotchCount::Double, NotchCount::Triple] {
            let offsets = offsets(count);
            let middle: f64 = offsets.iter().sum();
            assert!(middle.abs() < 1.0e-12, "{count:?} is off centre");
        }
        assert_eq!(offsets(NotchCount::Double).len(), 2);
        assert_eq!(offsets(NotchCount::Triple).len(), 3);
    }
}
