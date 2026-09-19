#![allow(
    clippy::float_cmp,
    reason = "every place here is a whole or a half centimetre, exact in a double"
)]

use toile_engine::draft::EdgeAnchor;

use super::super::tests::{pick, run, squares};
use super::*;

#[test]
fn the_nearest_tract_is_the_one_picked_and_only_within_reach() {
    let spread = squares();
    assert_eq!(under([10.3, 5.0], &spread, 0.5), Some(pick(&spread, 0, 1)));
    assert_eq!(under([29.8, 5.0], &spread, 0.5), Some(pick(&spread, 1, 3)));
    assert_eq!(under([20.0, 5.0], &spread, 0.5), None, "between the two");
    assert_eq!(under([5.0, 5.0], &spread, 0.5), None, "inside a piece");
}

#[test]
fn two_sides_that_run_the_same_way_down_the_mat_are_sewn_aligned() {
    let spread = squares();
    let line = |piece, tracts| line_of(&spread, run(&spread, piece, tracts)).expect("on the mat");
    let right = |piece| line(piece, [1, 1]);
    assert_eq!(facing(&right(0), &right(1)), SeamOrientation::Aligned);
    assert_eq!(
        facing(&right(0), &line(1, [3, 3])),
        SeamOrientation::Opposed
    );
    // A piece sewn to itself answers by the same rule: the top and the bottom
    // of one square run against each other, and left meets left.
    assert_eq!(
        facing(&line(0, [0, 0]), &line(0, [2, 2])),
        SeamOrientation::Opposed
    );
}

/// A run is judged by its two ends, so a side of several tracts that turns a
/// corner answers as the one tract between those ends would.
#[test]
fn a_run_of_several_tracts_faces_another_by_its_own_two_ends() {
    let spread = squares();
    let line = |piece, tracts| line_of(&spread, run(&spread, piece, tracts)).expect("on the mat");
    // Top, right side and bottom of the first square: from its top left round
    // to its bottom left, so downwards on the whole. The same run of the
    // second goes the same way; the second's left side alone goes upwards.
    let around = line(0, [0, 2]);
    assert_eq!(around.first(), Some(&[0.0, 0.0]));
    assert_eq!(around.last(), Some(&[0.0, 10.0]));
    assert_eq!(around.len(), 6, "every tract's own ends, in one line");
    assert_eq!(facing(&around, &line(1, [0, 2])), SeamOrientation::Aligned);
    assert_eq!(facing(&around, &line(1, [3, 3])), SeamOrientation::Opposed);
}

#[test]
fn a_tract_joins_a_run_only_at_its_ends_and_never_closes_the_contour() {
    let spread = squares();
    let right = pick(&spread, 0, 1);
    let tract = |index| pick(&spread, 0, index);
    assert_eq!(
        shifted(&spread, right, tract(2)),
        Ok(Some(run(&spread, 0, [1, 2])))
    );
    assert_eq!(
        shifted(&spread, right, tract(0)),
        Ok(Some(run(&spread, 0, [0, 1])))
    );
    assert_eq!(shifted(&spread, right, tract(3)), Err(APART));
    assert_eq!(shifted(&spread, right, pick(&spread, 1, 2)), Err(APART));
    let three = run(&spread, 0, [0, 2]);
    assert_eq!(
        shifted(&spread, three, tract(3)),
        Err(LOOP),
        "the fourth of four would end the run where it starts"
    );
}

#[test]
fn a_tract_leaves_a_run_only_from_its_ends() {
    let spread = squares();
    let three = run(&spread, 0, [0, 2]);
    let tract = |index| pick(&spread, 0, index);
    assert_eq!(
        shifted(&spread, three, tract(0)),
        Ok(Some(run(&spread, 0, [1, 2])))
    );
    assert_eq!(
        shifted(&spread, three, tract(2)),
        Ok(Some(run(&spread, 0, [0, 1])))
    );
    assert_eq!(shifted(&spread, three, tract(1)), Err(MIDDLE));
    assert_eq!(shifted(&spread, tract(1), tract(1)), Ok(None));
    assert!(holds(&spread, three, tract(1)));
    assert!(!holds(&spread, three, tract(3)));
    assert!(!holds(&spread, three, pick(&spread, 1, 1)));
}

#[test]
fn a_stretch_anchored_inside_its_tracts_is_drawn_from_there() {
    let spread = squares();
    let of = &spread[0];
    let part = EdgeRange {
        head: EdgeAnchor {
            piece: of.piece,
            from: of.tracts[1].node,
            t: 0.5,
        },
        tail: EdgeAnchor {
            piece: of.piece,
            from: of.tracts[2].node,
            t: 0.25,
        },
    };
    assert_eq!(Pick::of(part), None, "not a run of whole tracts");
    let line = stretch(&of.tracts, part);
    assert_eq!(line.first(), Some(&[10.0, 5.0]), "half way down the side");
    assert_eq!(line.last(), Some(&[7.5, 10.0]), "a quarter of the bottom");

    let within = EdgeRange {
        tail: EdgeAnchor {
            t: 0.75,
            ..part.head
        },
        ..part
    };
    assert_eq!(
        stretch(&of.tracts, within),
        vec![[10.0, 5.0], [10.0, 7.5]],
        "both ends on one tract"
    );
}

#[test]
fn a_run_with_no_length_or_no_piece_draws_nothing() {
    let spread = squares();
    let of = &spread[0];
    let pinched = Pick {
        piece: of.piece,
        from: of.tracts[1].node,
        to: of.tracts[1].node,
    };
    assert_eq!(
        line_of(&spread, pinched),
        None,
        "its two ends are one place"
    );
    assert_eq!(Pick::of(pinched.range()), Some(pinched));
    let elsewhere = Pick {
        piece: PieceKey::new(7, 0),
        ..pinched
    };
    assert_eq!(line_of(&spread, elsewhere), None, "a piece not on the mat");
}
