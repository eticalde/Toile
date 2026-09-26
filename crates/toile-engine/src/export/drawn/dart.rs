use super::Run;
use crate::draft::{Dart, Doc, Draft, FoldDirection, PieceKey};

/// How long each stroke of the cross at a dart's point is, in centimetres.
///
/// Centred on the apex, the way every count of notch is centred on the place it
/// names: a mark set to one side of the point would name a place half a
/// centimetre from the one the stitching has to stop at. Half of one stroke
/// therefore falls in the wedge, which is paper right up until the cutter takes
/// it off — and it is the wedge the mark is about.
const CROSS: f64 = 0.6;

/// How long the barb of the arrow that says which way the dart is pressed is.
const BARB: f64 = 0.35;

/// How far inside the cloth that arrow is drawn, in centimetres.
///
/// Both legs of a wedge sit on one straight tract, so the mouth is collinear
/// with the edge the wedge was taken out of and everything on the far side of
/// it is paper the cutter takes off. The arrow is therefore set in toward the
/// apex, and carries its one barb on that side too: a head splayed to both
/// sides of the mouth would have half of itself off the piece.
const INSET: f64 = 0.3;

/// Every dart on a piece, each as the marks it is drawn with.
pub(super) fn runs(draft: &Draft, piece: PieceKey) -> Vec<Vec<Run>> {
    let doc = draft.doc();
    doc.darts
        .iter()
        .filter(|&(_, held)| cut_into(doc, held) == Some(piece))
        .filter_map(|(_, held)| marks(draft, held))
        .collect()
}

/// The piece a dart is cut into, which is what its seam names.
///
/// The record names three points, and a point on its own does not name the
/// contour it is on. It is the same question `RemoveDart` asks of the document.
fn cut_into(doc: &Doc, dart: &Dart) -> Option<PieceKey> {
    doc.seams.get(dart.seam)?.a.piece()
}

/// The marks one dart is drawn with, and nothing at all when one of its three
/// points resolves nowhere.
///
/// The legs and the apex are contour nodes, so the cut line already draws the
/// wedge. What these add is the one thing the cut line cannot say: that the V
/// is not an opening. The mouth is drawn broken from leg to leg — the line the
/// finished edge runs along once the two legs are sewn to each other — the
/// arrow under it says which way the closed wedge is pressed, and the cross
/// says where the stitching stops.
fn marks(draft: &Draft, dart: &Dart) -> Option<Vec<Run>> {
    let apex = draft.resolved(dart.apex)?;
    let first = draft.resolved(dart.legs.0)?;
    let second = draft.resolved(dart.legs.1)?;
    let along = unit(first, second)?;
    let toward = toward_apex(along, first, apex);
    let middle = [0, 1].map(|axis| f64::midpoint(first[axis], second[axis]));
    let (tip, into) = match dart.fold {
        FoldDirection::TowardStart => (first, negated(along)),
        FoldDirection::TowardEnd => (second, along),
    };
    let mut out = vec![Run {
        at: vec![first, second],
        broken: true,
        label: None,
    }];
    out.push(Run::mark(arrow(tip, middle, into, toward)));
    out.push(Run::mark(stroke(apex, along)));
    out.push(Run::mark(stroke(apex, across(along))));
    Some(out)
}

/// The arrow from the middle of the mouth to the leg the folded wedge lies
/// against, set into the cloth and barbed on that side.
fn arrow(tip: [f64; 2], tail: [f64; 2], into: [f64; 2], toward: [f64; 2]) -> Vec<[f64; 2]> {
    let inset = |at: [f64; 2]| [0, 1].map(|axis| at[axis] + toward[axis] * INSET);
    let head = inset(tip);
    let barb = [0, 1].map(|axis| head[axis] - into[axis] * BARB + toward[axis] * BARB);
    vec![inset(tail), head, barb]
}

/// Which side of the mouth the cloth is on: the quarter turn that points at the
/// apex.
///
/// Read off the wedge itself and not off the winding the piece declares, the
/// way a notch reads its own direction off the resolved outline: what the mark
/// has to stay inside is the paper the scissors will leave.
fn toward_apex(along: [f64; 2], leg: [f64; 2], apex: [f64; 2]) -> [f64; 2] {
    let across = across(along);
    let side = (apex[0] - leg[0]) * across[0] + (apex[1] - leg[1]) * across[1];
    if side < 0.0 { negated(across) } else { across }
}

/// One stroke of the cross, centred on the apex and laid along `direction`.
fn stroke(apex: [f64; 2], direction: [f64; 2]) -> Vec<[f64; 2]> {
    let end = |side: f64| [0, 1].map(|axis| apex[axis] + direction[axis] * side * CROSS / 2.0);
    vec![end(-1.0), end(1.0)]
}

/// The direction from one place to another, and nothing when they are one
/// place.
fn unit(from: [f64; 2], to: [f64; 2]) -> Option<[f64; 2]> {
    let span = (to[0] - from[0]).hypot(to[1] - from[1]);
    (span > f64::EPSILON).then(|| [(to[0] - from[0]) / span, (to[1] - from[1]) / span])
}

/// A quarter turn of a direction.
fn across(direction: [f64; 2]) -> [f64; 2] {
    [-direction[1], direction[0]]
}

/// The other way along a direction.
fn negated(direction: [f64; 2]) -> [f64; 2] {
    [-direction[0], -direction[1]]
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "the mouth of the fixture runs along an axis of the grid, which is exact"
    )]

    use super::*;

    /// A mouth four centimetres wide running along +x.
    const MOUTH: ([f64; 2], [f64; 2]) = ([8.0, 0.0], [12.0, 0.0]);

    /// Where the apex is, on a wedge whose mouth is `MOUTH`.
    const APEX: [f64; 2] = [10.0, 5.0];

    /// The middle of that mouth, which is where the arrow starts from.
    const MIDDLE: [f64; 2] = [10.0, 0.0];

    /// The arrow points at the leg the dart is pressed against, and at the
    /// other one when it is pressed the other way.
    ///
    /// Which way a dart is pressed is the one thing about it a sewer cannot
    /// work out from the drawing, so an arrow that answered the same either
    /// way would leave the record saying something the paper does not.
    #[test]
    fn the_arrow_points_at_the_leg_the_wedge_is_pressed_against() {
        let along = unit(MOUTH.0, MOUTH.1).expect("four centimetres apart");
        let toward = toward_apex(along, MOUTH.0, APEX);
        let toward_start = arrow(MOUTH.0, MIDDLE, negated(along), toward);
        let toward_end = arrow(MOUTH.1, MIDDLE, along, toward);
        assert_eq!(toward_start[1][0], MOUTH.0[0]);
        assert_eq!(toward_end[1][0], MOUTH.1[0]);
        // And every place of either one is on the apex's side of the mouth,
        // which is the side the cloth is on: the far side is paper the cutter
        // takes off, and on a tiled sheet it is past the edge of the page.
        for at in toward_start.iter().chain(&toward_end) {
            assert!(at[1] > 0.0, "{at:?} is off the cloth");
        }
    }

    /// Which side the cloth is on is read off the wedge, so a dart cut into the
    /// bottom edge of a piece is marked inside it just the same.
    #[test]
    fn the_arrow_turns_toward_the_apex_whichever_side_it_is_on() {
        let along = unit(MOUTH.0, MOUTH.1).expect("four centimetres apart");
        assert_eq!(toward_apex(along, MOUTH.0, APEX), [0.0, 1.0]);
        let above = [APEX[0], -APEX[1]];
        assert_eq!(toward_apex(along, MOUTH.0, above), [0.0, -1.0]);
    }

    /// The cross is centred on the apex, both strokes, so the two of them name
    /// one place and that place is the point the stitching stops at.
    #[test]
    fn both_strokes_of_the_cross_are_centred_on_the_apex() {
        let apex = [10.0, 5.0];
        let along = unit(MOUTH.0, MOUTH.1).expect("four centimetres apart");
        for direction in [along, across(along)] {
            let run = stroke(apex, direction);
            let middle = [0, 1].map(|axis| f64::midpoint(run[0][axis], run[1][axis]));
            assert_eq!(middle, apex, "{run:?}");
            let span = (run[1][0] - run[0][0]).hypot(run[1][1] - run[0][1]);
            assert!((span - CROSS).abs() < 1.0e-12, "{span}");
        }
    }

    /// A wedge whose mouth is one place draws nothing rather than drawing a
    /// mark with no direction: the document refuses such a wedge, and a file
    /// hand-edited into one is not a reason to put ink nowhere.
    #[test]
    fn a_mouth_of_no_width_has_no_direction_to_draw_along() {
        assert!(unit(MOUTH.0, MOUTH.0).is_none());
    }
}
