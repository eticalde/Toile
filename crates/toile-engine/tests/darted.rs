#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_engine::draft::{
    Command, Dart, DartWedge, Doc, Draft, FoldDirection, Identity, MeasureSet, Piece, PieceKey,
    Point, SeamKey, WedgeNode, Winding,
};
use toile_engine::export::{A4, Inked, piece_to_pdf, to_svg};

/// A back panel with one dart cut into its top edge, in centimetres.
///
/// Sixteen by twelve, so the cloth takes one sheet of A4 and what is counted is
/// the ink and never the tiling.
const PANEL: [[f64; 2]; 4] = [[0.0, 0.0], [16.0, 0.0], [16.0, 12.0], [0.0, 12.0]];

/// The panel with a wedge four centimetres wide and five deep taken out of its
/// waistline, sewn shut and pressed toward the leg nearer the contour's start.
fn darted() -> (Draft, PieceKey) {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 78.0)]));
    let nodes: Vec<_> = PANEL
        .iter()
        .map(|&[x, y]| doc.points.insert(Point::at(x, y)))
        .collect();
    let piece = doc
        .pieces
        .insert(Piece::polygon("TRASERO", nodes.clone(), Winding::Cw));
    let node = |x: f64, y: f64| WedgeNode::line(Identity::New, Point::at(x, y));
    // Every key the dart names is one the edit itself issues, so only the fold
    // is read off here.
    let unknown = nodes[0];
    Command::AddDart {
        identity: Identity::New,
        dart: Dart {
            apex: unknown,
            legs: (unknown, unknown),
            seam: SeamKey::new(0, 0),
            fold: FoldDirection::TowardStart,
        },
        wedge: Box::new(DartWedge {
            piece,
            after: Some(nodes[0]),
            nodes: [node(6.0, 0.0), node(8.0, 5.0), node(10.0, 0.0)],
        }),
    }
    .apply(&mut doc)
    .expect("the wedge sits on the waistline");
    (Draft::from_doc(doc).expect("the panel resolves"), piece)
}

/// A cutter with the printed sheet sees the wedge and reads that it is sewn
/// shut: the mouth is drawn broken across it, an arrow inside the cloth says
/// which way the closed wedge is pressed, and a cross says where the stitching
/// stops.
///
/// Both sheets carry the same four marks in the same places, because both take
/// them from the one layer that works them out.
#[test]
fn a_dart_reaches_both_sheets_as_a_wedge_that_is_sewn_shut() {
    let (draft, piece) = darted();
    let drawing = to_svg(&draft).expect("the panel draws");
    let printed = piece_to_pdf(&draft, piece, A4).expect("the panel prints");
    assert_eq!(printed.sheets(), 1, "sixteen centimetres fit on one sheet");
    assert_eq!(
        printed.piles[0].pieces[0].inked,
        Inked {
            lines: 0,
            notches: 0,
            darts: 1,
            names: 7,
        }
    );
    for mark in [
        "M 60.00 0.00 L 100.00 0.00",
        "M 80.00 3.00 L 60.00 3.00 L 63.50 6.50",
        "M 77.00 50.00 L 83.00 50.00",
        "M 80.00 47.00 L 80.00 53.00",
    ] {
        assert!(drawing.contains(mark), "{mark} missing from {drawing}");
    }
    // The mouth is the one run of the four that is broken: nothing is cut along
    // it, because it is where the two legs meet once they are sewn.
    assert_eq!(drawing.matches("stroke-dasharray").count(), 1, "{drawing}");
    let stream = String::from_utf8_lossy(&printed.bytes).into_owned();
    let broken = stream
        .split_once("] 0 d\n")
        .expect("the sheet breaks the mouth")
        .1;
    let broken = broken.split_once("[] 0 d").expect("and turns it off").0;
    assert_eq!(broken.matches("\nS\n").count(), 1, "{broken}");
    // The shaft of the grain arrow and its four barbs, then the dart's own
    // arrow and the two strokes of its cross.
    assert_eq!(stream.matches("\nS\n").count(), 5 + 3 + 1, "{stream}");
}

/// Nothing of the wedge is drawn on the paper the cutter takes off.
///
/// The two legs sit on one straight tract, so the mouth lies along the very
/// edge the wedge came out of, and a mark on the far side of it would be on
/// paper that is about to be cut away — and, on a tiled sheet, past the edge of
/// the page. It is the same rule a notch is cut inward by.
#[test]
fn every_mark_of_a_dart_but_its_mouth_is_inside_the_cloth() {
    let (draft, _) = darted();
    let drawing = to_svg(&draft).expect("the panel draws");
    for at in drawing.split("M ").skip(1) {
        for place in at.split_once('"').expect("a path is quoted").0.split(" L ") {
            let y: f64 = place
                .split_whitespace()
                .nth(1)
                .expect("a place is two numbers")
                .parse()
                .expect("millimetres");
            assert!(y >= 0.0, "{place} is above the waistline of {drawing}");
        }
    }
}
