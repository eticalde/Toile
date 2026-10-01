#![allow(
    clippy::float_cmp,
    reason = "a tolerance reads back as exactly the number it was given"
)]

use toile_doc::{EdgeRange, Seam, SeamKind, block};

use super::{Lengths, side_cm, tolerance_cm};
use crate::draft::Draft;

/// The shipped block and the two seams it declares, in key order.
fn block() -> (Draft, Vec<Seam>) {
    let draft = Draft::from_doc(block::trousers()).expect("the block resolves");
    let seams = draft.doc().seams.iter().map(|(_, s)| *s).collect();
    (draft, seams)
}

/// Both sides of both of the block's seams measure what the drawing says, and
/// both are inside the tolerance the document carries.
///
/// The contrast every reading of a pattern that is *not* inside it is taken
/// against, so the numbers are here rather than in the report that prints them.
#[test]
fn the_shipped_block_sews_two_sides_of_the_same_length() {
    let (draft, seams) = block();
    assert_eq!(seams.len(), 2, "the block closes one leg with two seams");
    for seam in &seams {
        let lengths = Lengths::of(&draft, seam).expect("the block anchors all four ends");
        assert!(
            lengths.excess(seam) < tolerance_cm(&draft, seam),
            "the block's sides agree: {:.2} against {:.2} cm, out by {:.2}",
            lengths.a,
            lengths.b,
            lengths.delta()
        );
        assert_eq!(lengths.meets(&draft, seam), Some(true));
    }
}

/// A side is as long as the walk along its own contour, head to tail, and not
/// as long as the walk the other way round.
///
/// The closure is the whole of it: a side that runs past the point the contour
/// opens at wraps, so the walk is taken modulo the perimeter and never as a
/// signed difference. Measured on the block, the two directions differ by more
/// than a metre.
#[test]
fn a_side_measures_the_walk_between_its_two_ends_the_way_the_contour_runs() {
    let (draft, seams) = block();
    let seam = seams.first().expect("the block declares a seam");
    let measured = side_cm(&draft, &seam.a).expect("the side is anchored");
    let piece = seam.a.piece().expect("the side names one piece");
    let perimeter = draft.perimeter_cm(piece);
    assert!(
        measured > 0.0 && measured < perimeter,
        "a side is part of its own contour: {measured} of {perimeter}"
    );

    let back = EdgeRange::between(piece, seam.a.tail.from, seam.a.head.from);
    let other = side_cm(&draft, &back).expect("the same two nodes the other way");
    assert!(
        (measured + other - perimeter).abs() < 1.0e-6,
        "the two walks between the same two nodes are the whole contour: \
         {measured} and {other} against {perimeter}"
    );
}

/// A seam carrying its own tolerance is judged by that and not by the
/// document's variable, in both directions.
///
/// Both directions, because a tolerance that only ever widened would pass
/// every pattern: the block's own sides sit inside half a centimetre, so the
/// tight reading is the one that proves the number is being read at all.
#[test]
fn a_seam_is_judged_by_its_own_tolerance_before_the_documents() {
    let (draft, seams) = block();
    let mut seam = *seams.first().expect("the block declares a seam");
    let lengths = Lengths::of(&draft, &seam).expect("the block anchors all four ends");
    let out_by = lengths.excess(&seam);
    assert!(out_by > 0.0, "the block's two sides are not identical");

    seam.tolerance = Some(out_by / 2.0);
    assert_eq!(tolerance_cm(&draft, &seam), out_by / 2.0);
    assert_eq!(
        lengths.meets(&draft, &seam),
        Some(false),
        "a tolerance under the mismatch refuses it: {out_by:.4} cm out"
    );

    seam.tolerance = Some(out_by * 2.0);
    assert_eq!(
        lengths.meets(&draft, &seam),
        Some(true),
        "and one over it accepts the same seam"
    );
}

/// An eased seam is judged against the difference it asks for, so a mismatch
/// of exactly that size passes where a plain seam's would not.
#[test]
fn an_eased_seam_is_judged_against_the_ease_it_asks_for() {
    let (draft, seams) = block();
    let mut seam = *seams.first().expect("the block declares a seam");
    let lengths = Lengths::of(&draft, &seam).expect("the block anchors all four ends");
    let plain = lengths.excess(&seam);

    seam.kind = SeamKind::Eased {
        expected_cm: lengths.delta().abs(),
    };
    assert!(
        lengths.excess(&seam) < plain,
        "asking for the difference the sides have leaves nothing out: {:.4}",
        lengths.excess(&seam)
    );

    seam.kind = SeamKind::Eased { expected_cm: 4.0 };
    seam.tolerance = Some(0.5);
    assert_eq!(
        lengths.meets(&draft, &seam),
        Some(false),
        "and asking for four centimetres the sides do not have is a fault"
    );
}

/// A gathered seam renders no verdict, because the document does not say which
/// of its two sides is the gathered one.
#[test]
fn a_gathered_seam_is_not_judged_on_a_length_at_all() {
    let (draft, seams) = block();
    let mut seam = *seams.first().expect("the block declares a seam");
    let lengths = Lengths::of(&draft, &seam).expect("the block anchors all four ends");
    seam.kind = SeamKind::Gathered { ratio: 1.5 };
    assert_eq!(lengths.meets(&draft, &seam), None);
    assert_eq!(tolerance_cm(&draft, &seam), Seam::DEFAULT_RATIO_TOLERANCE);
}
