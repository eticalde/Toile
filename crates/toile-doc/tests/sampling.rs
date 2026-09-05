#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_doc::{Command, Doc, DocError, PieceKey, Point, PointKey, SegmentEdit, block};

/// How finely a tract bent in these tests is flattened.
const SAMPLES: u16 = 12;

fn front(doc: &Doc) -> PieceKey {
    doc.piece_named(block::FRONT).expect("the block draws one")
}

fn node(doc: &Doc, label: &str) -> PointKey {
    doc.shows_label(front(doc), label)
        .unwrap_or_else(|| panic!("the block names {label}"))
}

/// The edit that says how finely the tract leaving `at` is flattened.
fn sampling(piece: PieceKey, at: PointKey, to: u16) -> Command {
    Command::SetSamples {
        piece,
        node: at,
        to,
    }
}

/// The edit that hollows the tract leaving `at`, on two fresh handles.
fn curve(piece: PieceKey, at: PointKey) -> Command {
    Command::SetSegment {
        piece,
        node: at,
        to: SegmentEdit::cubic(Point::at(-2.6, 52.0), Point::at(-5.2, 38.0)),
    }
}

/// The block with the inseam above the knee already bent, and that node.
fn curved() -> (Doc, PointKey) {
    let mut doc = block::trouser_front();
    let knee = node(&doc, "rodilla_int");
    sampling(front(&doc), knee, SAMPLES)
        .apply(&mut doc)
        .expect("a straight tract takes any count under the ceiling");
    curve(front(&doc), knee)
        .apply(&mut doc)
        .expect("the tract is sampled for a curve");
    (doc, knee)
}

/// The count sizes the flattened contour, and the flattened contour is what
/// every resolve walks pairwise, twice over. A number outside what a tract
/// can carry is refused where it is written, not clamped where it is read.
#[test]
fn a_flattening_no_tract_can_carry_is_refused() {
    let (mut doc, knee) = curved();
    let before = doc.to_canonical_json();
    let piece = front(&doc);
    for count in [0, 1, 97, u16::MAX] {
        assert_eq!(
            sampling(piece, knee, count).apply(&mut doc),
            Err(DocError::sampling(count)),
            "{count} samples"
        );
    }
    // A straight tract gives its own node and stops, so one describes it.
    let waist = node(&doc, "cintura_cf");
    assert!(sampling(piece, waist, 1).apply(&mut doc).is_ok());
    assert_eq!(
        sampling(piece, waist, 97).apply(&mut doc),
        Err(DocError::sampling(97))
    );
    assert_eq!(doc.to_canonical_json(), before);
}

/// Bending is two edits and the count is the first of them: a cubic flattened
/// at one sample draws, meshes and measures as its own chord, so the tract
/// may not take handles until it is sampled finely enough to show them.
#[test]
fn a_tract_may_not_bend_until_it_is_sampled_for_a_curve() {
    let mut doc = block::trouser_front();
    let knee = node(&doc, "rodilla_int");
    let piece = front(&doc);
    let before = doc.to_canonical_json();
    assert_eq!(
        curve(piece, knee).apply(&mut doc),
        Err(DocError::sampling(1))
    );
    assert_eq!(doc.to_canonical_json(), before, "a refusal moves nothing");

    sampling(piece, knee, SAMPLES)
        .apply(&mut doc)
        .expect("a straight tract takes any count under the ceiling");
    assert!(curve(piece, knee).apply(&mut doc).is_ok());
}
