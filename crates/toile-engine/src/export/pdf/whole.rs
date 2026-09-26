use super::pack::GAP;
use super::paper::points_of_cm;
use super::read::{cuts_of, step, streams_of};
use super::tests::{ELL, polygon, rectangle};
use super::text::literal;
use super::{A4, SheetError, piece_to_pdf, to_pdf};
use crate::draft::{Doc, Draft, MeasureSet, Piece, Point, Winding, block};

/// A product of named rectangles, in centimetres.
fn laid(pieces: &[(&str, [f64; 2])]) -> Draft {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    for &(name, [wide, tall]) in pieces {
        let corners = [[0.0, 0.0], [wide, 0.0], [wide, tall], [0.0, tall]];
        let points: Vec<_> = corners
            .into_iter()
            .map(|[x, y]| doc.points.insert(Point::at(x, y)))
            .collect();
        doc.pieces.insert(Piece::polygon(name, points, Winding::Cw));
    }
    Draft::from_doc(doc).expect("rectangles resolve")
}

/// A product of three rectangles, named and sized so that what a sheet says is
/// enough to tell its pile from the other two.
///
/// The second is wider than the paper and the other two are not, so the file
/// under test has one pile that is tiled and two that are a sheet each — and
/// the tiled one in the middle, where a pile that ran past its own pages would
/// show. None of the three shares paper with another: the second would take the
/// first from one sheet to three, and the third would ride on one column of the
/// second's three.
fn product() -> Draft {
    laid(&[
        ("Delantero", [12.0, 14.0]),
        ("Pretina", [40.0, 4.0]),
        ("Bolsillo", [8.0, 9.0]),
    ])
}

/// The owner's waistband and the two straps that go round it: three pieces
/// between forty and fifty centimetres long, none of them six centimetres tall,
/// and nine sheets of paper between them for thirteen centimetres of cloth.
fn strips() -> Draft {
    laid(&[
        ("PRETINA", [47.5, 4.0]),
        ("TIRA PASACINTOS", [45.5, 3.6]),
        ("TIRA CADENA", [42.8, 5.6]),
    ])
}

/// The whole point of the file: a garment is one print job and not one per
/// piece, with each piece's sheets together and in the order the pieces are
/// drawn.
#[test]
fn one_file_carries_every_piece_with_its_own_sheets_together() {
    let printed = to_pdf(&product(), A4).expect("three rectangles print");
    let names: Vec<&str> = printed
        .piles
        .iter()
        .map(super::product::Pile::name)
        .collect();
    assert_eq!(names, ["Delantero", "Pretina", "Bolsillo"]);
    assert_eq!(printed.sheets(), 1 + 3 + 1);
    let pages: Vec<[usize; 2]> = printed
        .piles
        .iter()
        .map(|pile| [pile.first_page, pile.last_page()])
        .collect();
    assert_eq!(pages, [[1, 1], [2, 4], [5, 5]]);
    let file = String::from_utf8_lossy(&printed.bytes).into_owned();
    assert_eq!(file.matches("/Type /Page ").count(), 5, "{file}");
    assert!(file.contains("/Count 5"), "{file}");
}

/// A person holding the loose sheets has to be able to make the piles again,
/// and the only thing they can read is what is printed: every sheet of every
/// piece names its piece, the tiled ones and the single ones alike.
#[test]
fn every_sheet_of_every_piece_names_the_piece_it_belongs_to() {
    let printed = to_pdf(&product(), A4).expect("three rectangles print");
    let pages = streams_of(&printed.bytes);
    assert_eq!(pages.len(), 5);
    let mut said = Vec::new();
    for pile in &printed.piles {
        for sheet in 1..=pile.sheets {
            said.push(if pile.sheets == 1 {
                format!("Pieza «{}»", pile.name())
            } else {
                format!("Pieza «{}» · hoja {sheet} de {}", pile.name(), pile.sheets)
            });
        }
    }
    for (page, said) in pages.iter().zip(&said) {
        assert!(page.contains(&literal(said)), "{said} is not on its sheet");
    }
}

/// The point of the packing: three strips that each tiled three columns of
/// their own now tile three columns between them, and nothing else changed.
#[test]
fn three_strips_of_one_waistband_come_out_on_three_sheets_instead_of_nine() {
    let printed = to_pdf(&strips(), A4).expect("three strips print");
    assert_eq!(printed.piles.len(), 1, "one stack of paper");
    assert_eq!(printed.pieces(), 3);
    assert_eq!(printed.sheets(), 3);
    assert_eq!(printed.piles[0].grid, [3, 1]);
    assert_eq!(printed.alone(), 9, "three sheets apiece on their own");
    assert_eq!(printed.saved(), 6);
}

/// A sheet carrying three pieces has to say which line belongs to which, or the
/// six sheets it saved are worth nothing: every sheet of the pile names every
/// piece on it, beside that piece's own outline, and says what it measures.
#[test]
fn every_sheet_of_a_shared_pile_names_every_piece_on_it() {
    let printed = to_pdf(&strips(), A4).expect("three strips print");
    let pages = streams_of(&printed.bytes);
    assert_eq!(pages.len(), 3);
    for (rank, page) in pages.iter().enumerate() {
        for (name, size) in [
            ("PRETINA", "47.5 × 4.0 cm"),
            ("TIRA PASACINTOS", "45.5 × 3.6 cm"),
            ("TIRA CADENA", "42.8 × 5.6 cm"),
        ] {
            let said = literal(&format!("«{name}»"));
            assert!(page.contains(&said), "{name} is not on sheet {}", rank + 1);
            assert!(page.contains(&literal(size)), "{name} says no size");
        }
        // And which stack of paper it is, so the three sheets can be found
        // again among forty loose ones.
        let titled = format!("Pliego «PRETINA» y otras 2 piezas · hoja {} de 3", rank + 1);
        assert!(page.contains(&literal(&titled)), "{page}");
    }
}

/// The blank paper between two pieces reaches the paper and not just the grid.
///
/// Measured the way the scissors meet it: every cut line read back off the
/// page, and the closest any two of them come compared with the gap the packing
/// is told to leave. What it buys is that a cut that wanders a millimetre on
/// both lines still leaves two whole pieces.
#[test]
fn two_pieces_that_share_a_sheet_are_a_gap_apart_on_the_paper() {
    let printed = to_pdf(&strips(), A4).expect("three strips print");
    for (rank, page) in streams_of(&printed.bytes).iter().enumerate() {
        let cuts = cuts_of(page);
        assert_eq!(cuts.len(), 3, "sheet {} carries all three", rank + 1);
        let mut nearest = f64::INFINITY;
        for (rank, one) in cuts.iter().enumerate() {
            for other in &cuts[rank + 1..] {
                for &at in one {
                    for &other in other {
                        nearest = nearest.min(step(at, other));
                    }
                }
            }
        }
        let apart = nearest / points_of_cm(1.0) * 10.0;
        assert!(
            apart >= GAP - 0.01,
            "{apart} mm apart on sheet {}",
            rank + 1
        );
    }
}

/// One piece asked for by name shares its paper with nothing, because nothing
/// else was asked for: the file is the one it was before any packing existed.
#[test]
fn one_piece_asked_for_by_name_never_shares_its_paper() {
    let draft = strips();
    let piece = draft.doc().piece_keys()[0];
    let printed = piece_to_pdf(&draft, piece, A4).expect("the waistband tiles");
    assert_eq!(printed.pieces(), 1);
    assert_eq!(printed.sheets(), 3);
    assert_eq!(printed.saved(), 0);
    let page = &streams_of(&printed.bytes)[0];
    assert!(
        page.contains(&literal("Pieza «PRETINA» · hoja 1 de 3")),
        "{page}"
    );
    assert!(!page.contains(&literal("TIRA CADENA")), "{page}");
    // And no name beside the outline: with one piece on the sheet the legend's
    // own title already says which it is.
    assert!(!page.contains(&literal("«PRETINA»")), "{page}");
}

/// Two pieces under one name are two piles a person cannot separate, since the
/// name is what every sheet of a pile has in common and nothing else is. The
/// file is still written: what has to change is the name.
#[test]
fn two_piles_under_one_name_are_named_as_the_pair_they_are() {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    for _ in 0..2 {
        let points: Vec<_> = [[0.0, 0.0], [12.0, 0.0], [12.0, 14.0], [0.0, 14.0]]
            .into_iter()
            .map(|[x, y]| doc.points.insert(Point::at(x, y)))
            .collect();
        doc.pieces
            .insert(Piece::polygon("Delantero", points, Winding::Cw));
    }
    let draft = Draft::from_doc(doc).expect("two rectangles resolve");
    let printed = to_pdf(&draft, A4).expect("both of them print");
    assert_eq!(printed.sheets(), 2);
    assert_eq!(printed.twinned(), ["Delantero"]);
    assert!(
        to_pdf(&product(), A4)
            .expect("three rectangles print")
            .twinned()
            .is_empty()
    );
}

/// A product whose pieces are all printable leaves none out, which is the
/// baseline the next test is read against.
#[test]
fn the_owners_kind_of_product_leaves_no_piece_out() {
    let printed = to_pdf(&product(), A4).expect("three rectangles print");
    assert!(printed.left_out.is_empty());
    let block = Draft::from_doc(block::trousers()).expect("the block resolves");
    let printed = to_pdf(&block, A4).expect("the block prints");
    assert!(printed.left_out.is_empty(), "{:?}", printed.left_out);
    assert_eq!(printed.pieces(), block.doc().piece_keys().len());
}

/// One bad piece must not cost a person the other nine: the file is written
/// without it and the piece is named, because a refusal would leave them with
/// the same broken formula and no paper.
#[test]
fn a_piece_that_cannot_be_printed_does_not_take_the_others_with_it() {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    let corner = doc.points.insert(Point::at(0.0, 0.0));
    doc.pieces
        .insert(Piece::polygon("Punto", vec![corner], Winding::Cw));
    let points: Vec<_> = [[0.0, 0.0], [12.0, 0.0], [12.0, 14.0], [0.0, 14.0]]
        .into_iter()
        .map(|[x, y]| doc.points.insert(Point::at(x, y)))
        .collect();
    doc.pieces
        .insert(Piece::polygon("Delantero", points, Winding::Cw));
    let draft = Draft::from_doc(doc).expect("a point and a rectangle resolve");
    let printed = to_pdf(&draft, A4).expect("the rectangle prints without the point");
    assert_eq!(printed.sheets(), 1);
    assert_eq!(printed.piles[0].name(), "Delantero");
    assert_eq!(printed.piles[0].first_page, 1, "the pile opens the file");
    assert_eq!(printed.left_out.len(), 1);
    assert_eq!(printed.left_out[0].name, "Punto");
    assert_eq!(printed.left_out[0].why, SheetError::Empty);
}

/// A product no piece of which reaches paper writes no file, and says so about
/// every piece: one name would have the person fix one piece and print nothing
/// again.
#[test]
fn a_product_that_reaches_no_paper_names_every_piece_that_stopped_it() {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    for name in ["Punto", "Otro punto"] {
        let corner = doc.points.insert(Point::at(0.0, 0.0));
        doc.pieces
            .insert(Piece::polygon(name, vec![corner], Winding::Cw));
    }
    let draft = Draft::from_doc(doc).expect("two one-point pieces resolve");
    let refused = to_pdf(&draft, A4).expect_err("no piece of it reaches paper");
    let named: Vec<&str> = refused
        .left_out
        .iter()
        .map(|piece| piece.name.as_str())
        .collect();
    assert_eq!(named, ["Punto", "Otro punto"]);
}

/// One piece asked for on its own is the file it was before a product could be
/// printed whole: the same bytes, so the person who re-cut one panel gets what
/// he got last time.
#[test]
fn one_piece_asked_for_on_its_own_is_the_file_it_always_was() {
    let (draft, piece) = rectangle(12.0, 14.0);
    let alone = piece_to_pdf(&draft, piece, A4).expect("a rectangle prints");
    let whole = to_pdf(&draft, A4).expect("a product of one rectangle prints");
    assert_eq!(alone.bytes, whole.bytes);
    assert_eq!(alone.piles, whole.piles);
}

/// The same product writes the same file. The other half of this proof runs in
/// two processes, which is the only way to catch an address or a clock.
#[test]
fn the_same_product_writes_the_same_file_twice() {
    let once = to_pdf(&product(), A4).expect("it prints");
    let again = to_pdf(&product(), A4).expect("it prints");
    assert_eq!(once, again);
    let (draft, _) = polygon(&ELL);
    assert_eq!(to_pdf(&draft, A4), to_pdf(&draft, A4));
}
