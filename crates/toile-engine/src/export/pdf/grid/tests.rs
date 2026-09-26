use super::super::A4;
use super::super::pack::Placed;
use super::super::tests::{ELL, polygon};
use super::*;

/// A rectangle of cloth, in centimetres, hung at the document's origin.
fn cloth(wide: f64, tall: f64) -> Vec<[f64; 2]> {
    vec![[0.0, 0.0], [wide, 0.0], [wide, tall], [0.0, tall]]
}

/// One outline as the only piece of a pile, at the corner of its own plane.
fn pile(outline: &[[f64; 2]]) -> Vec<Placed> {
    let (_, piece) = polygon(outline);
    vec![Placed::new(piece, outline.to_vec())]
}

/// The sheet a piece that fits takes, and the one thing that must not have
/// changed for it: it is still one sheet, and the cloth still sits on it
/// where a sheet with no grid put it.
#[test]
fn a_piece_that_fits_is_one_sheet_and_the_grid_is_a_no_op() {
    let grid = Grid::new(A4, &pile(&cloth(12.0, 14.0))).expect("it fits");
    assert_eq!(grid.counts(), [1, 1]);
    let places = grid.places();
    assert_eq!(places.len(), 1);
    assert_eq!(
        places[0].joins,
        Joins {
            above: None,
            below: None,
            left: None,
            right: None
        }
    );
    let spare = (cell(A4)[0] - 120.0) / 2.0;
    let [x, y] = grid.cloths()[0].cut()[0];
    assert!((x - spare).abs() < 1.0e-9, "{x} against {spare}");
    assert!(y.abs() < 1.0e-9, "{y}");
}

/// Each sheet after the first advances by a step and not by a cell, so the
/// count is the one a person can check by laying the sheets out.
#[test]
fn a_piece_wider_than_the_paper_takes_a_column_for_every_step() {
    let cell = cell(A4)[0];
    let step = cell - OVERLAP;
    for (extra, want) in [(0.0, 1), (0.1, 2), (step, 2), (step + 0.1, 3)] {
        let wide = (cell + extra) / 10.0;
        let grid = Grid::new(A4, &pile(&cloth(wide, 1.0))).expect("it fits a ream");
        assert_eq!(grid.counts()[0], want, "{wide} cm");
    }
}

/// A size that lands a nanometre past a whole step is that step, and not
/// another sheet of paper for a millionth of a millimetre of cloth.
#[test]
fn a_nanometre_past_a_whole_step_is_not_another_sheet() {
    let cell = cell(A4)[0];
    let step = cell - OVERLAP;
    let wide = (cell + step + 1.0e-10) / 10.0;
    let grid = Grid::new(A4, &pile(&cloth(wide, 1.0))).expect("it fits");
    assert_eq!(grid.counts()[0], 2);
}

/// The sheets are numbered in the order they are printed, and a cell with
/// no cloth on it takes no number at all.
#[test]
fn the_numbers_run_in_reading_order_and_skip_the_blank_cells() {
    let grid = Grid::new(A4, &pile(&ELL)).expect("it fits");
    assert_eq!(grid.counts(), [2, 2]);
    let places = grid.places();
    assert_eq!(places.len(), 3, "the hollow cell prints nothing");
    assert_eq!(places[0].cell, [0, 0]);
    assert_eq!(places[0].number, 1);
    assert_eq!(places[0].joins.right, None, "the sheet beside it is blank");
    assert_eq!(places[0].joins.below, Some(2));
    assert_eq!(places[1].cell, [0, 1]);
    assert_eq!(places[1].joins.above, Some(1));
    assert_eq!(places[2].cell, [1, 1]);
    assert_eq!(places[2].joins.left, Some(2));
    assert_eq!(places[2].joins.above, None);
}

/// A piece that asks for more paper than a ream is a formula that slipped a
/// decimal, and it is said in sheets because that is what it would cost.
#[test]
fn a_piece_that_asks_for_more_than_a_ream_is_refused_in_sheets() {
    let refused = Grid::new(A4, &pile(&cloth(1_000.0, 1_000.0)));
    let Err(SheetError::TooMany {
        sheets, size_cm, ..
    }) = refused
    else {
        panic!("ten metres of cloth each way is not a pattern: {refused:?}");
    };
    assert!(sheets > REAM, "{sheets}");
    assert!((size_cm[0] - 1_000.0).abs() < 1.0e-9, "{size_cm:?}");
    assert!((size_cm[1] - 1_000.0).abs() < 1.0e-9, "{size_cm:?}");
}

/// A pile of one piece is one pile by definition, and the hollow of a concave
/// piece does not change that: the sheets that are printed all carry it.
#[test]
fn a_pile_of_one_piece_is_carried_by_every_sheet_of_itself() {
    assert!(Grid::new(A4, &pile(&ELL)).expect("it fits").one_pile());
}

/// Two pieces on one plane that do not both reach every sheet of it are two
/// piles, however little paper they take between them: fifteen sheets of a
/// stack of eighteen would carry one of them and three would carry both.
#[test]
fn two_pieces_that_do_not_share_every_sheet_are_not_one_pile() {
    let (_, tall) = polygon(&cloth(12.0, 40.0));
    let (_, short) = polygon(&cloth(12.0, 4.0));
    let mut beside = Placed::new(short, cloth(12.0, 4.0));
    beside.place = [0.0, 300.0];
    let laid = vec![Placed::new(tall, cloth(12.0, 40.0)), beside];
    let grid = Grid::new(A4, &laid).expect("it fits");
    assert_eq!(grid.counts(), [1, 2], "forty centimetres is two sheets");
    assert!(
        !grid.one_pile(),
        "the short piece is on the second sheet only"
    );
}
