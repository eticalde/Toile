#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_engine::draft::{
    Command, Doc, Draft, EdgeAnchor, EdgeRange, Identity, LineEdit, LineKind, MeasureSet, Notch,
    NotchCount, Piece, PieceKey, Point, Symmetry, VertexEdit, Winding,
};
use toile_engine::export::{A4, Inked, to_pdf, to_svg};

/// Half a waistband on the fold, in centimetres: nine drawn and eighteen cut.
///
/// Small enough that the cloth takes one sheet of A4, so what is counted here
/// is the ink and never the tiling.
const BAND: [[f64; 2]; 4] = [[0.0, 0.0], [9.0, 0.0], [9.0, 4.0], [0.0, 4.0]];

/// Where the crease is: the tract leaving the second node, which is the right
/// edge of the drawn half.
const CREASE: usize = 1;

/// The band with a fold line and a belt-loop mark drawn on it, a notch cut in
/// its top edge, and, when it is asked for, the axis it is folded on.
///
/// The four marks a cutter reads are on one piece on purpose: what is being
/// proved is that they all reach both sheets, and a fixture apiece would prove
/// it about four different pieces.
fn band(folded: bool) -> (Draft, PieceKey) {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 78.0)]));
    let nodes: Vec<_> = BAND
        .iter()
        .map(|&[x, y]| doc.points.insert(Point::at(x, y)))
        .collect();
    let piece = doc
        .pieces
        .insert(Piece::polygon("PRETINA", nodes.clone(), Winding::Cw));
    for (kind, across, name) in [
        (LineKind::Placement, 3.0, "presilla"),
        (LineKind::Fold, 6.0, "doblez"),
    ] {
        let place = |y| VertexEdit::free(Point::at(across, y));
        let line = LineEdit::new(piece, kind, place(1.0))
            .to(place(3.0))
            .named(name);
        Command::AddLine {
            identity: Identity::New,
            line: Box::new(line),
        }
        .apply(&mut doc)
        .expect("a place of its own needs nothing of the contour");
    }
    Command::AddNotch {
        identity: Identity::New,
        notch: Notch {
            at: EdgeAnchor {
                piece,
                from: nodes[0],
                t: 0.5,
            },
            mate: None,
            count: NotchCount::Single,
        },
        mate: None,
    }
    .apply(&mut doc)
    .expect("the middle of the top edge");
    if folded {
        let axis = EdgeRange::between(piece, nodes[CREASE], nodes[CREASE + 1]);
        Command::AddSymmetry {
            identity: Identity::New,
            symmetry: Symmetry::fold(axis),
        }
        .apply(&mut doc)
        .expect("both ends are nodes of the band");
    }
    (Draft::from_doc(doc).expect("the band resolves"), piece)
}

/// The drawing and the sheet carry the same marks in the same places, which is
/// the whole of what one dialect buys: a cutter who prints the pattern and a
/// cutter who opens the drawing are cutting the same garment.
///
/// The places are read in millimetres off the drawing, where the unit is the
/// millimetre. The sheet is measured in its own test, in points, against the
/// same centimetres.
#[test]
fn the_drawing_and_the_sheet_carry_the_same_marks() {
    let (folded, piece) = band(true);
    let drawing = to_svg(&folded).expect("the band draws");
    let printed = to_pdf(&folded, piece, A4).expect("the band prints");
    assert_eq!(printed.sheets, 1, "eighteen centimetres fit on one sheet");
    assert_eq!(
        printed.inked,
        Inked {
            lines: 2,
            notches: 1,
            names: 4,
        }
    );
    // Every mark of the drawn half, and every one of them again on the half the
    // fold mirrors: the belt-loop mark at three centimetres and at fifteen, the
    // fold line at six and at twelve, and the notch cut half a centimetre into
    // the cloth at four and a half and at thirteen and a half.
    for at in ["30.00", "150.00"] {
        let mark = format!("M {at} 10.00 L {at} 30.00");
        assert!(drawing.contains(&mark), "{mark} missing from {drawing}");
    }
    for at in ["45.00", "135.00"] {
        let cut = format!("M {at} 0.00 L {at} 5.00");
        assert!(drawing.contains(&cut), "{cut} missing from {drawing}");
    }
    let stream = String::from_utf8_lossy(&printed.bytes).into_owned();
    // The grain arrow is a shaft and four barbs; the two lines and the notch
    // are one run each on the drawn half and one on the mirrored one.
    assert_eq!(stream.matches("\nS\n").count(), 5 + 2 * 3, "{stream}");
    assert_eq!(
        drawing.matches("<path ").count(),
        1 + 5 + 2 * 3,
        "{drawing}"
    );
}

/// A name the pattern holds reaches both sheets the way each of them carries a
/// name that is not part of the drawing: the node names are inked on both, and
/// the name of a line is inked on neither.
#[test]
fn a_name_the_pattern_holds_reaches_both_sheets() {
    let (folded, piece) = band(true);
    let drawing = to_svg(&folded).expect("the band draws");
    let printed = to_pdf(&folded, piece, A4).expect("the band prints");
    let stream = String::from_utf8_lossy(&printed.bytes).into_owned();
    for name in ["presilla", "doblez"] {
        assert_eq!(
            drawing.matches(&format!("<title>{name}</title>")).count(),
            2,
            "{name} is not named on both halves of the drawing"
        );
        assert!(
            !stream.contains(&format!("({name}) Tj")),
            "{name} is inked on the sheet, where the mark itself says which it is"
        );
    }
    // The piece's own name, and the four nodes the drawing lets a hand take
    // hold of: the mirrored half has no nodes of its own to name.
    assert!(drawing.contains("<title>PRETINA</title>"), "{drawing}");
    assert!(stream.contains("(Pieza \\253PRETINA\\273)"), "{stream}");
    for node in ["P1", "P2", "P3", "P4"] {
        assert!(drawing.contains(&format!(">{node}</text>")), "{drawing}");
        assert_eq!(stream.matches(&format!("({node}) Tj")).count(), 1, "{node}");
    }
}

/// Which lines are broken is the pattern's own decision, so the two sheets come
/// to one answer: the fold line is broken on both and the belt-loop mark is
/// whole on both.
///
/// A fold a cutter cannot tell from a placement line is a garment cut wrong,
/// and a fold that is broken in one export and whole in the other is worse: it
/// is two patterns from one document.
#[test]
fn the_two_sheets_break_the_same_lines() {
    let (drawn, piece) = band(false);
    let drawing = to_svg(&drawn).expect("the band draws");
    let printed = to_pdf(&drawn, piece, A4).expect("the band prints");
    let stream = String::from_utf8_lossy(&printed.bytes).into_owned();
    // The drawing breaks one path, and it is the one named after the fold.
    assert_eq!(drawing.matches("stroke-dasharray").count(), 1, "{drawing}");
    let dashed = drawing
        .split_once("stroke-dasharray")
        .expect("the drawing breaks a line")
        .1;
    assert!(
        dashed.starts_with("=\"4.00 2.00\"><title>doblez"),
        "{dashed}"
    );
    // The sheet turns its dashes on once, draws the one broken run, and turns
    // them off again.
    let broken = stream
        .split_once("] 0 d\n")
        .expect("the sheet breaks a line")
        .1;
    let broken = broken.split_once("[] 0 d").expect("and turns them off").0;
    assert_eq!(broken.matches("\nS\n").count(), 1, "{broken}");
}

/// The sheet says whose measurements the pattern was drawn on. Forty loose
/// sheets of one garment say nothing about who is going to wear it, and the
/// same piece resolved against another body is another pattern on the same
/// paper.
#[test]
fn the_sheet_says_which_body_the_pattern_was_drawn_on() {
    let (drawn, piece) = band(false);
    let printed = to_pdf(&drawn, piece, A4).expect("the band prints");
    let stream = String::from_utf8_lossy(&printed.bytes).into_owned();
    assert!(
        stream.contains("(Trazada para \\253Etienne\\273.)"),
        "{stream}"
    );
}
