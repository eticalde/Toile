#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_engine::couture;
use toile_engine::draft::{
    Binding, Command, Doc, Draft, MeasureSet, Piece, PieceKey, Point, PointKey, Recompile,
    SegmentEdit, Winding, to_metres,
};

/// How finely the bowed tract is flattened.
const SAMPLES: u16 = 24;

/// The corners of a metre-square piece, in centimetres and contour order.
const CORNERS: [[f64; 2]; 4] = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];

/// The square with the tract leaving its second corner bowed out to the right.
///
/// Two topology edits build it: the one that says how finely the tract is
/// flattened and the one that draws the curve, in that order, which is what
/// the curve tool emits inside a single gesture.
fn bowed() -> (Draft, PieceKey, Vec<PointKey>) {
    let mut doc = Doc::new(MeasureSet::new("Etienne", []));
    let nodes: Vec<PointKey> = CORNERS
        .iter()
        .map(|&[x, y]| doc.points.insert(Point::at(x, y)))
        .collect();
    let piece = doc
        .pieces
        .insert(Piece::polygon("Delantero", nodes.clone(), Winding::Cw));
    let mut draft = Draft::from_doc(doc).expect("a square resolves");
    draft
        .edit(Command::SetSamples {
            piece,
            node: nodes[1],
            to: SAMPLES,
        })
        .expect("the tract takes a sample count");
    draft
        .edit(Command::SetSegment {
            piece,
            node: nodes[1],
            to: SegmentEdit::cubic(Point::at(130.0, 33.0), Point::at(130.0, 67.0)),
        })
        .expect("the tract takes a curve");
    (draft, piece, nodes)
}

/// The handle leaving the bowed node.
fn handle(draft: &Draft, piece: PieceKey) -> PointKey {
    draft
        .doc()
        .pieces
        .get(piece)
        .expect("the key is live")
        .contour[1]
        .segment
        .handles()
        .expect("the tract bends")
        .0
}

/// The piece's nodes in metres: the contour it would have had without curves.
fn chords(draft: &Draft, piece: PieceKey) -> Vec<[f64; 2]> {
    draft
        .points_cm(piece)
        .iter()
        .map(|&(_, at)| to_metres(at))
        .collect()
}

#[test]
fn a_curve_flattens_into_the_contour_the_mesher_takes() {
    let (draft, piece, _) = bowed();
    let expected = 3 + usize::from(SAMPLES);
    assert!(draft.defects(piece).is_empty());
    assert_eq!(draft.points_cm(piece).len(), 4, "the nodes are still four");
    assert_eq!(draft.outline(piece).len(), expected);
    assert_eq!(draft.flat_cm(piece).len(), expected);
    assert_eq!(draft.topology(piece), 2, "the curve and its sample count");
}

/// An edit that creates entities issues keys, and an arena never hands an
/// index out twice. Applying the command to see whether it resolves and then
/// applying it again to record it would spend two slots per handle and leave
/// the first pair burned, so a file written after four bends would claim
/// twice the points it carries.
#[test]
fn bending_a_tract_through_the_draft_issues_one_key_per_handle() {
    let (draft, piece, _) = bowed();
    let doc = draft.doc();
    let (out, into) = doc.pieces.get(piece).expect("the key is live").contour[1]
        .segment
        .handles()
        .expect("the tract bends");
    assert_eq!(doc.points.len(), 6, "four corners and two handles");
    assert_eq!(doc.points.issued(), 6, "and not a slot more");
    assert_eq!((out.index(), into.index()), (4, 5));
}

#[test]
fn the_run_along_a_bowed_tract_is_longer_than_its_chord() {
    let (draft, piece, nodes) = bowed();
    let run = draft.run_length_cm(piece, nodes[1], nodes[2]);
    assert!(run > 105.0, "the bowed tract measures {run} cm");
    assert!(
        draft.perimeter_cm(piece) > 405.0,
        "the chords measure 400 cm"
    );
}

#[test]
fn moving_a_handle_re_derives_without_moving_the_topology_counter() {
    let (mut draft, piece, _) = bowed();
    let handle = handle(&draft, piece);
    let before = draft.outline(piece).to_vec();

    draft.begin_gesture("mover manija");
    let what = draft
        .edit(Command::MovePoint {
            point: handle,
            to: [Binding::literal(190.0), Binding::literal(10.0)],
        })
        .expect("the document takes the move");
    draft.end_gesture();

    assert_eq!(what, Recompile::Shape(vec![piece]));
    assert_eq!(draft.topology(piece), 2, "a tangent is not a topology");
    assert_eq!(draft.outline(piece).len(), before.len());
    assert_ne!(draft.outline(piece), before.as_slice());
}

#[test]
fn straightening_a_tract_is_a_topology_edit_and_undo_puts_the_curve_back() {
    let (mut draft, piece, nodes) = bowed();
    let curved = draft.outline(piece).to_vec();
    let what = draft
        .edit(Command::SetSegment {
            piece,
            node: nodes[1],
            to: SegmentEdit::Line,
        })
        .expect("the tract straightens");

    assert_eq!(what, Recompile::Topology(vec![piece]));
    assert_eq!(draft.outline(piece).len(), 4);
    assert_eq!(draft.topology(piece), 3);
    assert_eq!(draft.undo(), Ok(Recompile::Topology(vec![piece])));
    assert_eq!(draft.outline(piece), curved.as_slice());
}

#[test]
fn the_mesh_density_is_read_off_the_flattening() {
    let (draft, piece, _) = bowed();
    let along = couture::for_contour(draft.outline(piece)).0;
    let across = couture::for_contour(&chords(&draft, piece)).0;
    assert!(
        along > across,
        "{along} samples along the arc, {across} across"
    );
}
