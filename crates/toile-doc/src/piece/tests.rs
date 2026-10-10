#![allow(clippy::float_cmp, reason = "a grain stores the angle it was given")]

use super::*;

fn keys() -> Vec<PointKey> {
    (0..4).map(|index| PointKey::new(index, 0)).collect()
}

#[test]
fn a_polygon_is_all_straight_tracts() {
    let piece = Piece::polygon("Delantero", keys(), Winding::Cw);
    assert_eq!(piece.contour.len(), 4);
    assert!(piece.contour.iter().all(|node| node.samples == 1));
    assert!(
        piece
            .contour
            .iter()
            .all(|node| node.segment == Segment::Line)
    );
    assert_eq!(piece.grain, Grain::VERTICAL);
}

#[test]
fn a_node_is_found_by_its_point_never_by_an_index() {
    let keys = keys();
    let piece = Piece::polygon("Delantero", keys.clone(), Winding::Cw);
    assert_eq!(piece.node_index(keys[2]), Some(2));
    assert_eq!(piece.node_index(PointKey::new(9, 0)), None);
}

#[test]
fn a_handle_is_cited_by_the_piece_that_holds_its_tract() {
    let keys = keys();
    let mut piece = Piece::polygon("Delantero", keys.clone(), Winding::Cw);
    let handle = PointKey::new(7, 0);
    piece.contour[1].segment = Segment::Cubic {
        out: handle,
        into: keys[3],
    };
    assert!(piece.cites(handle));
    assert!(!piece.cites(PointKey::new(8, 0)));
    assert_eq!(piece.node_index(handle), None);
}

#[test]
fn only_a_bending_tract_needs_more_than_one_sample() {
    let mut node = ContourNode::line(PointKey::new(0, 0));
    assert!(node.takes_samples(1));
    assert!(node.takes_samples(SAMPLES.1));
    assert!(!node.takes_samples(0));
    assert!(!node.takes_samples(SAMPLES.1 + 1));

    node.segment = Segment::Cubic {
        out: PointKey::new(4, 0),
        into: PointKey::new(5, 0),
    };
    assert!(!node.takes_samples(1));
    assert!(node.takes_samples(SAMPLES.0));
    assert!(!node.takes_samples(u16::MAX));
}

#[test]
fn only_an_arranged_piece_asks_for_the_placement_s_format() {
    let mut piece = Piece::polygon("Delantero", keys(), Winding::Cw);
    assert_eq!(piece.placement, None);
    assert_eq!(piece.format_version(), crate::json::VERSION);
    piece.placement = Some(Placement::new(3.0, -1.5));
    assert_eq!(piece.format_version(), crate::json::VERSION_PLACED);
}

/// A fresh piece says nothing about being cut, so it stays in the version it
/// was in. Each of the four says something on its own, and the newest number
/// any one of them asks for is the one the whole piece asks for.
#[test]
fn a_piece_that_says_how_it_is_cut_asks_for_the_cut_s_format() {
    let plain = Piece::polygon("Delantero", keys(), Winding::Cw);
    assert!(!plain.says_how_it_is_cut());
    assert_eq!(plain.format_version(), crate::json::VERSION);

    let saying: [fn(&mut Piece); 4] = [
        |piece| piece.seam_allowance = Some(1.5),
        |piece| piece.quantity = 2,
        |piece| piece.letter = Some("A".to_owned()),
        |piece| piece.labels = vec!["cortar 2 espejadas".to_owned()],
    ];
    for say in saying {
        let mut piece = plain.clone();
        piece.placement = Some(Placement::new(3.0, -1.5));
        say(&mut piece);
        assert!(piece.says_how_it_is_cut());
        assert_eq!(piece.format_version(), crate::json::VERSION_CUT);
    }
}

/// The allowance is a width of cloth outside the line: zero is a piece cut on
/// its line, and everything below zero takes cloth off it. A count of one is
/// the count a file leaves out, and zero is no count at all.
#[test]
fn the_numbers_no_piece_is_cut_by_are_refused() {
    assert_eq!(check_allowance(None), Ok(()));
    assert_eq!(check_allowance(Some(0.0)), Ok(()));
    assert_eq!(check_allowance(Some(1.5)), Ok(()));
    for width in [-0.001, -1.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            check_allowance(Some(width)),
            Err(DocError::SeamAllowance),
            "{width}"
        );
    }
    assert_eq!(check_quantity(0), Err(DocError::CutQuantity));
    for count in [Piece::CUT_ONCE, 2, u32::MAX] {
        assert_eq!(check_quantity(count), Ok(()), "{count}");
    }
}

#[test]
fn a_clockwise_contour_on_the_page_has_a_positive_area() {
    assert_eq!(Winding::of_area(5230.39), Winding::Cw);
    assert_eq!(Winding::of_area(-1.0), Winding::Ccw);
}
