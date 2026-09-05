#![allow(missing_docs, reason = "a test crate publishes no API surface")]
#![allow(
    clippy::float_cmp,
    reason = "a node the split did not move comes back bit for bit or not at all"
)]

use std::collections::BTreeMap;

use toile_doc::{
    Command, Doc, History, Identity, MeasureSet, Piece, PieceKey, Point, PointKey, Segment,
    SegmentEdit, Winding,
};
use toile_geom::curve;

/// How finely the two tracts of the split are flattened.
const SAMPLES: u16 = 24;

/// A piece with one bending tract, and the nodes it runs through.
///
/// Everything is bound to plain numbers, so the drawing is the document read
/// straight off: a formula would put the resolved geometry in the engine's
/// hands, and this is a test of the edit rather than of the resolver.
fn bent() -> (Doc, PieceKey, Vec<PointKey>) {
    let mut doc = Doc::new(MeasureSet::default());
    let points: Vec<PointKey> = [[0.0, 0.0], [14.0, 2.0], [14.0, 20.0], [0.0, 20.0]]
        .into_iter()
        .map(|[x, y]| doc.points.insert(Point::at(x, y)))
        .collect();
    let piece = doc
        .pieces
        .insert(Piece::polygon("Delantero", points.clone(), Winding::Cw));
    Command::SetSamples {
        piece,
        node: points[0],
        to: SAMPLES,
    }
    .apply(&mut doc)
    .expect("the contour runs through the node");
    Command::SetSegment {
        piece,
        node: points[0],
        to: SegmentEdit::cubic(Point::at(3.0, 8.0), Point::at(11.0, -6.0)),
    }
    .apply(&mut doc)
    .expect("the tract is sampled for a curve a count ago");
    (doc, piece, points)
}

/// Where a point sits, in centimetres.
fn at(doc: &Doc, key: PointKey) -> [f64; 2] {
    let held = doc.points.get(key).expect("the key is live");
    let plain = |binding: &toile_doc::Binding| {
        binding
            .eval(&BTreeMap::new())
            .expect("this document is bound to plain numbers")
    };
    [plain(&held.x), plain(&held.y)]
}

/// The control net of the tract leaving `node`, in contour order.
fn net(doc: &Doc, piece: PieceKey, node: PointKey) -> [[f64; 2]; 4] {
    let held = doc.pieces.get(piece).expect("the key is live");
    let seat = held.node_index(node).expect("the contour runs through it");
    let tract = held.contour[seat];
    let Segment::Cubic { out, into } = tract.segment else {
        panic!("the tract bends")
    };
    let next = held.contour[(seat + 1) % held.contour.len()].point;
    [at(doc, node), at(doc, out), at(doc, into), at(doc, next)]
}

/// The two edits that cut the tract leaving `node` in two at `t`.
///
/// This is what the drawing tool will emit, in the order it emits it: the
/// tract that stays keeps its node and takes the first half of the split, and
/// the new node opens the second. Both halves come from de Casteljau, so the
/// line the two draw is the line the one drew.
fn cut(doc: &Doc, piece: PieceKey, node: PointKey, t: f64) -> [Command; 2] {
    let [p0, c1, c2, p1] = net(doc, piece, node);
    let (first, second) = curve::subdivide(p0, c1, c2, p1, t);
    [
        Command::SetSegment {
            piece,
            node,
            to: SegmentEdit::cubic(
                Point::at(first[1][0], first[1][1]),
                Point::at(first[2][0], first[2][1]),
            ),
        },
        Command::InsertNode {
            piece,
            after: Some(node),
            identity: Identity::New,
            value: Point::at(second[0][0], second[0][1]),
            segment: SegmentEdit::cubic(
                Point::at(second[1][0], second[1][1]),
                Point::at(second[2][0], second[2][1]),
            ),
            samples: SAMPLES,
        },
    ]
}

#[test]
fn inserting_a_node_on_a_curve_keeps_the_shape_within_1e_9() {
    let (mut doc, piece, points) = bent();
    let was = net(&doc, piece, points[0]);
    let mut history = History::new();
    history.begin("insert a node");
    for command in cut(&doc, piece, points[0], 0.4) {
        history
            .edit(&mut doc, command)
            .expect("the tract bends and the contour runs through its node");
    }
    history.end();

    let inserted = doc.pieces.get(piece).expect("the key is live").contour[1].point;
    let halves = [net(&doc, piece, points[0]), net(&doc, piece, inserted)];
    // Every point of the line that was drawn is still on the line that is
    // drawn, to a nanometre: the node count moved and the shape did not.
    for q in curve::flatten(was[0], was[1], was[2], was[3], 64) {
        let off = halves
            .iter()
            .map(|n| curve::nearest(n[0], n[1], n[2], n[3], q).1)
            .fold(f64::INFINITY, f64::min);
        assert!(off < 1.0e-9, "{off}");
    }
}

#[test]
fn the_node_lands_on_the_curve_and_the_tracts_meet_there() {
    let (mut doc, piece, points) = bent();
    let was = net(&doc, piece, points[0]);
    for command in cut(&doc, piece, points[0], 0.4) {
        command
            .apply(&mut doc)
            .expect("the tract bends and the contour runs through its node");
    }
    let inserted = doc.pieces.get(piece).expect("the key is live").contour[1].point;
    let (first, second) = (net(&doc, piece, points[0]), net(&doc, piece, inserted));
    assert_eq!(first[3], second[0], "the tracts meet on the new node");
    assert_eq!(second[3], was[3], "and the far node has not moved");
    let (_, off) = curve::nearest(was[0], was[1], was[2], was[3], at(&doc, inserted));
    assert!(off < 1.0e-9, "{off}");
}

#[test]
fn undoing_the_cut_gives_back_one_tract_and_its_two_handles() {
    let (mut doc, piece, points) = bent();
    let was = net(&doc, piece, points[0]);
    let handles = doc.pieces.get(piece).expect("the key is live").contour[0]
        .segment
        .handles()
        .expect("the block bends the first tract");
    let count = doc.points.len();

    let mut history = History::new();
    history.begin("insert a node");
    for command in cut(&doc, piece, points[0], 0.4) {
        history
            .edit(&mut doc, command)
            .expect("the tract bends and the contour runs through its node");
    }
    history.end();
    // A node and the two handles of the tract leaving it: the first half took
    // over the keys the whole tract's handles had.
    assert_eq!(doc.points.len(), count + 3);
    let held = doc.pieces.get(piece).expect("the key is live");
    assert_eq!(held.contour.len(), 5);

    history.undo(&mut doc).expect("one gesture, one entry");
    let held = doc.pieces.get(piece).expect("the key is live");
    assert_eq!(held.contour.len(), 4);
    assert_eq!(held.contour[0].segment.handles(), Some(handles));
    assert_eq!(held.contour[0].samples, SAMPLES);
    assert_eq!(net(&doc, piece, points[0]), was);
    assert_eq!(doc.points.len(), count);
}
