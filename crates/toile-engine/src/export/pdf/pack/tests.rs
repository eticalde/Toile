use super::super::tests::polygon;
use super::super::{A4, CARTA};
use super::*;

/// A rectangle of cloth, in centimetres, at the document's origin.
fn cloth(wide: f64, tall: f64) -> Vec<[f64; 2]> {
    vec![[0.0, 0.0], [wide, 0.0], [wide, tall], [0.0, tall]]
}

/// That rectangle as a piece ready for paper.
fn ready(wide: f64, tall: f64) -> Placed {
    let outline = cloth(wide, tall);
    let (_, piece) = polygon(&outline);
    Placed::new(piece, outline)
}

/// Where a piece was placed, in micrometres, which is finer than any printer
/// resolves and coarse enough to compare without a tolerance.
fn at(placed: &Placed) -> [i64; 2] {
    placed.place.map(|mm| (mm * 1_000.0).round() as i64)
}

/// How much paper each pile of a packing takes.
fn sheets(paper: Paper, ready: Vec<Placed>) -> Vec<usize> {
    piles(paper, ready)
        .iter()
        .map(|pile| {
            Grid::new(paper, pile)
                .expect("a pile a piece joined was laid out")
                .sheets()
        })
        .collect()
}

/// The owner's waistband, his belt-loop strap and his chain strap: three pieces
/// between forty and fifty centimetres long and none of them six tall, which on
/// either paper tile three columns each and leave four fifths of every sheet
/// blank. Nine sheets of paper for thirteen centimetres of cloth becomes three.
#[test]
fn three_strips_of_one_waistband_share_one_pile() {
    let laid = vec![ready(47.5, 4.0), ready(45.5, 3.6), ready(42.8, 5.6)];
    for paper in [A4, CARTA] {
        let piled = piles(paper, laid.clone());
        assert_eq!(piled.len(), 1, "{}", paper.name);
        assert_eq!(piled[0].len(), 3, "{}", paper.name);
        assert_eq!(sheets(paper, laid.clone()), vec![3], "{}", paper.name);
    }
}

/// A piece only ever joins a pile it costs nothing, so no piece ever loses
/// paper to a neighbour: two that each fill most of a sheet stay on two.
#[test]
fn a_piece_that_would_cost_the_pile_a_sheet_gets_a_pile_of_its_own() {
    let laid = vec![ready(18.0, 20.0), ready(18.0, 20.0)];
    assert_eq!(sheets(A4, laid), vec![1, 1]);
}

/// A panel two sheets tall and a strap that would ride on the second of them
/// and not the first: the paper would be free and the stack would stop being
/// one thing, so the strap takes a sheet of its own.
///
/// This is the case the rule exists for. The same arithmetic on the owner's own
/// jeans would hang his waistband off the bottom row of an eighteen-sheet back
/// leg, where fifteen sheets carry a leg and three carry a leg and a waistband.
#[test]
fn a_piece_that_would_reach_only_some_of_the_pile_gets_a_pile_of_its_own() {
    let laid = vec![ready(12.0, 30.0), ready(12.0, 3.0)];
    assert_eq!(sheets(A4, laid), vec![2, 1]);
}

/// A shelf is filled before another is opened: the fly facing and the button
/// panel go side by side on one sheet, where stacking them would have asked for
/// a second.
#[test]
fn two_small_pieces_go_beside_each_other_and_not_one_under_the_other() {
    let piled = piles(A4, vec![ready(5.0, 17.0), ready(8.0, 17.0)]);
    assert_eq!(piled.len(), 1);
    assert_eq!(at(&piled[0][0]), [0, 0]);
    assert_eq!(at(&piled[0][1]), [(50.0 + GAP) as i64 * 1_000, 0]);
    assert_eq!(
        sheets(A4, vec![ready(5.0, 17.0), ready(8.0, 17.0)]),
        vec![1]
    );
}

/// The piles come in the order the pattern draws their first pieces, so a
/// product printed today has its panels where it had them yesterday — and a
/// small piece drawn last rides on the paper of the piece it belongs with
/// instead of opening a pile at the end of the file.
#[test]
fn a_pile_opens_where_its_first_piece_would_have_been() {
    let laid = vec![
        ready(30.0, 100.0),
        ready(47.5, 4.0),
        ready(30.0, 100.0),
        ready(45.5, 3.6),
    ];
    let piled = piles(A4, laid);
    assert_eq!(piled.len(), 3);
    assert_eq!(piled[0].len(), 1, "a leg shares its paper with nothing");
    assert_eq!(piled[1].len(), 2, "the strap joined the waistband");
    assert_eq!(piled[2].len(), 1);
}

/// One piece is one pile, laid where a plane with no packing on it put it.
#[test]
fn one_piece_on_its_own_stays_at_the_corner_of_its_own_plane() {
    let piled = piles(A4, vec![ready(12.0, 14.0)]);
    assert_eq!(piled.len(), 1);
    assert_eq!(at(&piled[0][0]), [0, 0]);
}
