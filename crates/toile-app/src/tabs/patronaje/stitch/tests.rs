#![allow(
    clippy::float_cmp,
    reason = "every place here is a whole or a half centimetre, exact in a double"
)]

use toile_engine::draft::{EdgeAnchor, PieceKey, PointKey};

use super::*;

/// A ten centimetre square at `x`, drawn top, right side, bottom, left side.
fn square(index: u32, x: f64) -> Spread {
    let corners = [[x, 0.0], [x + 10.0, 0.0], [x + 10.0, 10.0], [x, 10.0]];
    let tracts = (0..4)
        .map(|k| Tract {
            node: PointKey::new(index * 4 + k as u32, 0),
            to: PointKey::new(index * 4 + (k as u32 + 1) % 4, 0),
            line: vec![corners[k], corners[(k + 1) % 4]],
        })
        .collect();
    Spread {
        piece: PieceKey::new(index, 0),
        tracts,
    }
}

/// The stretch of a square from the head of one tract to the head of another.
fn run(of: &Spread, head: usize, tail: usize) -> EdgeRange {
    EdgeRange::between(of.piece, of.tracts[head].node, of.tracts[tail].node)
}

#[test]
fn an_opposed_seam_draws_its_second_side_tail_to_head() {
    let spread = [square(0, 0.0), square(1, 30.0)];
    // The right side of the first runs down; the left side of the second up.
    let (right, left) = (run(&spread[0], 1, 2), run(&spread[1], 3, 0));
    let aligned = Seam::plain(right, left, SeamOrientation::Aligned);
    let [a, b] = sides(&spread, &aligned).expect("both sides are on the mat");
    assert_eq!(a, vec![[10.0, 0.0], [10.0, 10.0]]);
    assert_eq!(b, vec![[30.0, 10.0], [30.0, 0.0]], "as its contour runs");

    let opposed = Seam::plain(right, left, SeamOrientation::Opposed);
    let [a, b] = sides(&spread, &opposed).expect("both sides are on the mat");
    assert_eq!(a, vec![[10.0, 0.0], [10.0, 10.0]], "side A never turns");
    assert_eq!(
        b,
        vec![[30.0, 0.0], [30.0, 10.0]],
        "the pairing walks it from its tail, top first, like side A"
    );
}

#[test]
fn a_stretch_over_several_tracts_follows_the_contour_through_them() {
    let spread = [square(0, 0.0)];
    let line = stretch(&spread[0].tracts, run(&spread[0], 1, 3));
    let turns = line.clone();
    assert_eq!(turns.first(), Some(&[10.0, 0.0]));
    assert_eq!(turns.last(), Some(&[0.0, 10.0]));
    assert!(turns.contains(&[10.0, 10.0]), "round the corner: {turns:?}");
}

#[test]
fn a_stretch_anchored_inside_its_tracts_is_drawn_from_there() {
    let spread = [square(0, 0.0)];
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
    let line = stretch(&of.tracts, part);
    assert_eq!(line.first(), Some(&[10.0, 5.0]), "half way down the side");
    assert_eq!(
        line.last(),
        Some(&[7.5, 10.0]),
        "a quarter along the bottom"
    );

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
fn a_side_with_no_length_or_no_piece_draws_nothing() {
    let spread = [square(0, 0.0), square(1, 30.0)];
    let right = run(&spread[0], 1, 2);
    let pinched = run(&spread[1], 3, 3);
    let seam = Seam::plain(right, pinched, SeamOrientation::Aligned);
    assert_eq!(
        sides(&spread, &seam),
        None,
        "the engine pairs nothing there"
    );
    let elsewhere = Seam::plain(right, run(&square(7, 0.0), 0, 1), SeamOrientation::Aligned);
    assert_eq!(sides(&spread, &elsewhere), None, "a piece not on the mat");
}
