use super::super::drawn::Inked;
use super::paper::points_of_cm;
use super::read::{anchor_of, clip_of, step, stream_of};
use super::text::literal;
use super::{A4, piece_to_pdf};
use crate::draft::{
    Binding, Command, Doc, Draft, EdgeAnchor, Identity, LineEdit, LineKind, MeasureSet, Notch,
    NotchCount, Piece, PieceKey, Point, VertexEdit, Winding,
};

/// The sides of the marked piece, in centimetres: one sheet of A4, so that what
/// is proved here is the ink and never the tiling.
const WIDE: f64 = 12.0;
const TALL: f64 = 14.0;

/// How deep a notch's mark is cut, in centimetres, as the sheet is read back.
const DEEP: f64 = 0.5;

/// A piece carrying one of everything a cutter reads: four named corners, a
/// line of every kind there is, and a single, a double and a triple notch on
/// three different sides of it.
///
/// One fixture and not one per mark, because what is being proved is that they
/// all reach the same sheet.
fn marked() -> (Draft, PieceKey) {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    let corners = [[0.0, 0.0], [WIDE, 0.0], [WIDE, TALL], [0.0, TALL]];
    let points: Vec<_> = corners
        .iter()
        .zip(["A", "B", "C", "D"])
        .map(|(&[x, y], name)| {
            doc.points.insert(Point {
                x: Binding::Literal(x),
                y: Binding::Literal(y),
                label: Some(name.to_owned()),
                label_visible: true,
            })
        })
        .collect();
    let piece = doc
        .pieces
        .insert(Piece::polygon("Cuadro", points.clone(), Winding::Cw));
    for (rank, kind) in kinds().into_iter().enumerate() {
        let across = 1.0 + rank as f64;
        let place = |y| VertexEdit::free(Point::at(across, y));
        let mut edit = LineEdit::new(piece, kind, place(2.0)).to(place(12.0));
        if kind == LineKind::Fold {
            edit = edit.named("doblez");
        }
        Command::AddLine {
            identity: Identity::New,
            line: Box::new(edit),
        }
        .apply(&mut doc)
        .expect("a place of its own needs nothing of the contour");
    }
    for (rank, count) in [NotchCount::Single, NotchCount::Double, NotchCount::Triple]
        .into_iter()
        .enumerate()
    {
        Command::AddNotch {
            identity: Identity::New,
            notch: Notch {
                at: EdgeAnchor {
                    piece,
                    from: points[rank],
                    t: 0.5,
                },
                mate: None,
                count,
            },
            mate: None,
        }
        .apply(&mut doc)
        .expect("the middle of a side of the piece");
    }
    (Draft::from_doc(doc).expect("the piece resolves"), piece)
}

/// Every kind of internal line there is, so that adding one to the document is
/// a compile error here.
fn kinds() -> Vec<LineKind> {
    vec![
        LineKind::Fold,
        LineKind::Stitch,
        LineKind::Slit,
        LineKind::Buttonhole,
        LineKind::Placement,
        LineKind::Reference,
    ]
}

/// Everything the document draws on the piece is on the sheet, and the sheet
/// says so in the numbers the command prints before the owner spends paper.
#[test]
fn every_mark_the_document_carries_reaches_the_sheet() {
    let (draft, piece) = marked();
    let printed = piece_to_pdf(&draft, piece, A4).expect("the marked piece prints");
    assert_eq!(
        printed.piles[0].pieces[0].inked,
        Inked {
            lines: 6,
            notches: 3,
            darts: 0,
            names: 4,
        }
    );
    let stream = stream_of(&printed.bytes);
    let (whole, dashed) = marks(&stream);
    // The shaft of the grain arrow and its four barbs, six notch marks, and the
    // three lines of a kind that is drawn whole.
    assert_eq!(whole.len(), 5 + 6 + 3, "{stream}");
    assert_eq!(dashed.len(), 3, "a fold, a stitch and a reference line");
}

/// A fold a cutter cannot tell from a slit is a garment cut wrong, so the sheet
/// breaks the same lines the drawing breaks — and only those.
#[test]
fn only_the_kinds_the_pattern_breaks_are_drawn_broken() {
    let (draft, piece) = marked();
    let printed = piece_to_pdf(&draft, piece, A4).expect("the marked piece prints");
    let stream = stream_of(&printed.bytes);
    let (whole, dashed) = marks(&stream);
    // Every line is ten centimetres long and each runs down its own column of
    // the piece, in the order the kinds were drawn, so the leftmost of the six
    // is the first kind and which set it came back in says how it was drawn.
    let ten = points_of_cm(10.0);
    let drawn = |run: &&Vec<[f64; 2]>| run.len() == 2 && (step(run[0], run[1]) - ten).abs() < 0.02;
    let across = |run: &Vec<[f64; 2]>| run[0][0];
    let mut every: Vec<f64> = whole
        .iter()
        .chain(&dashed)
        .filter(drawn)
        .map(across)
        .collect();
    every.sort_by(f64::total_cmp);
    assert_eq!(every.len(), 6, "every line the piece is drawn with");
    let broken: Vec<f64> = dashed.iter().filter(drawn).map(across).collect();
    for (rank, kind) in kinds().into_iter().enumerate() {
        let dashes = broken.iter().any(|&at| (at - every[rank]).abs() < 1.0e-9);
        assert_eq!(
            dashes,
            matches!(
                kind,
                LineKind::Fold | LineKind::Stitch | LineKind::Reference
            ),
            "{kind:?}"
        );
    }
}

/// A notch is cut into the cloth and never out of it, and the page measures its
/// mark in the centimetres the document asked for.
///
/// The three notches sit on three different sides of the piece, so the three
/// directions here are the one claim that matters: inward is inward whichever
/// way the contour happens to be running.
#[test]
fn a_notch_cuts_into_the_cloth_from_the_side_it_is_marked_on() {
    let (draft, piece) = marked();
    let printed = piece_to_pdf(&draft, piece, A4).expect("the marked piece prints");
    let stream = stream_of(&printed.bytes);
    let (whole, _) = marks(&stream);
    let deep = points_of_cm(DEEP);
    let cuts: Vec<[f64; 2]> = whole
        .iter()
        .filter(|run| run.len() == 2 && (step(run[0], run[1]) - deep).abs() < 0.02)
        .map(|run| [run[1][0] - run[0][0], run[1][1] - run[0][1]])
        .collect();
    assert_eq!(cuts.len(), 1 + 2 + 3, "one mark, then two, then three");
    // The page measures upward and the draft downward, so a mark cut into the
    // cloth from the top side of the piece runs down the page.
    let along = |want: [f64; 2]| cuts.iter().filter(|cut| step(**cut, want) < 0.02).count();
    assert_eq!(along([0.0, -deep]), 1, "the single notch cuts downward");
    assert_eq!(along([-deep, 0.0]), 2, "the double notch cuts leftward");
    assert_eq!(along([0.0, deep]), 3, "the triple notch cuts upward");
}

/// The names reach the paper as type: the nodes' own, the piece's, and the body
/// the pattern was drawn on.
#[test]
fn the_sheet_says_every_name_a_cutter_reads() {
    let (draft, piece) = marked();
    let printed = piece_to_pdf(&draft, piece, A4).expect("the marked piece prints");
    let stream = stream_of(&printed.bytes);
    for name in ["A", "B", "C", "D"] {
        let said = format!("{} Tj", literal(name));
        assert!(stream.contains(&said), "{name} is not on the sheet");
    }
    assert!(
        stream.contains(&literal("Trazada para «Etienne».")),
        "{stream}"
    );
    assert!(stream.contains(&literal("Pieza «Cuadro»")), "{stream}");
}

/// And every one of them is set where the sheet still shows it.
///
/// The case the test above cannot see, and the one that happened: a name is
/// written up and to the right of its node, a node on the piece's own top edge
/// sits on the cell's own boundary, and the clip discards the whole line. The
/// bytes carry it and the paper does not — so `contains` passes and fifteen of
/// the owner's sixty-four names reached no sheet of his file. This fixture's
/// corners A and B are exactly that node, which is why the test above
/// green-lit them.
#[test]
fn every_name_is_set_inside_the_clip_that_shows_it() {
    let (draft, piece) = marked();
    let printed = piece_to_pdf(&draft, piece, A4).expect("the marked piece prints");
    let stream = stream_of(&printed.bytes);
    let [left, low, wide, tall] = clip_of(&stream);
    let mut seen = 0;
    for name in ["A", "B", "C", "D"] {
        let [x, y] = anchor_of(&stream, &literal(name));
        assert!(
            x >= left && x <= left + wide && y >= low && y <= low + tall,
            "«{name}» is set at {x},{y}, outside the clip {left},{low} {wide}×{tall}"
        );
        seen += 1;
    }
    assert_eq!(seen, 4, "every corner was read");
}

/// A sheet of the marked piece is as deterministic as a blank one: every mark
/// on it comes out of the document and nothing out of an address or a clock.
#[test]
fn the_same_marked_piece_writes_the_same_sheet_twice() {
    let (draft, piece) = marked();
    let once = piece_to_pdf(&draft, piece, A4).expect("it prints");
    let again = piece_to_pdf(&draft, piece, A4).expect("it prints");
    assert_eq!(once, again);
}

/// Every open run of one stretch of a content stream, in points.
type Runs = Vec<Vec<[f64; 2]>>;

/// The runs the sheet strokes inside the piece's own clip, whole ones and
/// broken ones apart, in points.
///
/// Read from the end of the cut line to the close of the clip, which is the
/// span of the stream the piece's marks are written in — so a mark that escaped
/// the clip and could land on a neighbouring sheet's paper is not counted here
/// at all.
fn marks(stream: &str) -> (Runs, Runs) {
    let inside = stream
        .split_once("h S\n")
        .expect("the sheet carries a cut line")
        .1;
    let inside = inside.split_once("\nQ\n").expect("the clip closes").0;
    let Some((whole, broken)) = inside.split_once("] 0 d\n") else {
        return (runs(inside), Vec::new());
    };
    (
        runs(whole),
        runs(broken.split_once("[] 0 d").map_or(broken, |split| split.0)),
    )
}

/// Every open run of a stretch of the stream, in the order it is stroked.
fn runs(stream: &str) -> Runs {
    let mut out = Vec::new();
    let mut run: Vec<[f64; 2]> = Vec::new();
    for line in stream.lines() {
        let word: Vec<&str> = line.split_whitespace().collect();
        let place = |x: &str, y: &str| Some([x.parse().ok()?, y.parse().ok()?]);
        match word.as_slice() {
            [x, y, "m"] => run = place(x, y).map(|at| vec![at]).unwrap_or_default(),
            [x, y, "l"] => run.extend(place(x, y)),
            ["S"] => out.push(std::mem::take(&mut run)),
            _ => {}
        }
    }
    out
}
