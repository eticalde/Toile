#![allow(
    clippy::float_cmp,
    reason = "a place on the mat is the very number the point resolved to"
)]

use toile_engine::draft::{
    Command, Doc, EdgeAnchor, Identity, InternalLine, LineEdit, LineKey, LineKind, LineSpan,
    LineVertex, Notch, NotchCount, PieceKey, Point, PointKey, Segment, SegmentEdit, block,
};

use super::*;
use crate::tabs::patronaje::curve::SAMPLES;
use crate::tabs::patronaje::gesture::Stack;
use crate::tabs::patronaje::input::tests::{Table, table_of};

/// The shipped front with one line of `kind` drawn between two of its named
/// nodes, and the key that line took.
pub(in crate::tabs::patronaje) fn drawn(kind: LineKind, ends: [&str; 2]) -> (Doc, LineKey) {
    let mut doc = block::trouser_front();
    let piece = doc.piece_named(block::FRONT).expect("the block draws one");
    let node = |label: &str| {
        doc.shows_label(piece, label)
            .unwrap_or_else(|| panic!("the block names {label}"))
    };
    let place = |label| LineVertex::Contour(EdgeAnchor::at_node(piece, node(label)));
    let edit = LineEdit::new(piece, kind, place(ends[0])).to(place(ends[1]));
    let key = LineKey::new(doc.lines.issued(), 0);
    Command::AddLine {
        identity: Identity::New,
        line: Box::new(edit),
    }
    .apply(&mut doc)
    .expect("both places are nodes of the front");
    (doc, key)
}

/// A place on the run of the one line the table drew, half way along it.
fn on_the_line(table: &Table) -> [f64; 2] {
    let run = &table.drawn()[0].run;
    let k = (run.len() - 1) / 2;
    [0, 1].map(|axis| f64::midpoint(run[k][axis], run[k + 1][axis]))
}

#[test]
fn a_line_is_drawn_from_the_places_it_names_and_numbered_among_its_piece_s() {
    let (doc, key) = drawn(LineKind::Fold, ["cintura_cf", "cadera_lat"]);
    let table = table_of(doc);
    let lines = table.drawn();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].line, key);
    assert_eq!(lines[0].ordinal, 1, "the first line of the piece");
    assert_eq!(lines[0].kind, LineKind::Fold);
    let run = &lines[0].run;
    assert_eq!(run.len(), 2, "a straight span is its two ends");
    let node = |label: &str| {
        let doc = table.draft.doc();
        let key = doc.shows_label(table.piece, label).expect("named");
        table.draft.resolved(key).expect("it resolves")
    };
    assert_eq!(run[0], node("cintura_cf"));
    assert_eq!(run[1], node("cadera_lat"));
}

/// A line whose places no longer resolve draws nothing at all, and keeps its
/// number so the panel can still name it.
///
/// Nothing is thrown: a key that leads nowhere is bad input and reaches the mat
/// every time a file is hand-edited.
#[test]
fn a_line_whose_place_resolves_nowhere_draws_nothing_and_still_counts() {
    let (mut doc, _) = drawn(LineKind::Slit, ["cintura_cf", "cadera_lat"]);
    let piece = doc.piece_named(block::FRONT).expect("the block draws one");
    let adrift = LineVertex::Contour(EdgeAnchor::at_node(piece, PointKey::new(9999, 0)));
    doc.lines.insert(InternalLine {
        piece,
        kind: LineKind::Stitch,
        label: Some("pespunte suelto".to_owned()),
        head: adrift,
        spans: vec![LineSpan {
            to: adrift,
            segment: Segment::Line,
            samples: 1,
        }],
    });
    let table = table_of(doc);
    let lines = table.drawn();
    assert_eq!(lines.len(), 2, "both are listed");
    assert_eq!(lines[1].ordinal, 2);
    assert!(lines[1].run.is_empty(), "and the adrift one draws nothing");
    assert_eq!(lines[1].label.as_deref(), Some("pespunte suelto"));
}

#[test]
fn the_line_under_the_pointer_is_found_on_it_and_not_beside_it() {
    let (doc, key) = drawn(LineKind::Placement, ["cintura_cf", "cadera_lat"]);
    let table = table_of(doc);
    let at = on_the_line(&table);
    assert_eq!(under(at, table.drawn(), 0.2), Some(key));
    let off = [at[0], at[1] + 5.0];
    assert_eq!(under(off, table.drawn(), 0.2), None, "five centimetres off");
    assert_eq!(under(off, table.drawn(), 6.0), Some(key), "a wide reach");
}

/// Only the lines of the piece in front are drawn on it.
#[test]
fn a_line_of_another_piece_is_not_drawn_on_this_one() {
    let (doc, _) = drawn(LineKind::Fold, ["cintura_cf", "cadera_lat"]);
    let table = table_of(doc);
    let elsewhere = PieceKey::new(9999, 0);
    let lines = of(&table.draft, elsewhere, &[], [0.0, 0.0]);
    assert!(lines.is_empty(), "{lines:?}");
}

/// A notch is drawn where it sits along its tract, with the way the contour
/// runs there, so the cut can be laid across it.
#[test]
fn a_notch_is_placed_along_its_tract_and_carries_the_way_it_runs() {
    let mut doc = block::trouser_front();
    let piece = doc.piece_named(block::FRONT).expect("the block draws one");
    let from = doc.shows_label(piece, "cintura_cf").expect("named");
    let key = doc.notches.insert(Notch {
        count: NotchCount::Double,
        ..Notch::lone(EdgeAnchor {
            piece,
            from,
            t: 0.5,
        })
    });
    let table = table_of(doc);
    let tracts = tract::of(&table.draft, table.piece);
    let ticks = ticks(&table.draft, table.piece, &tracts);
    assert_eq!(ticks.len(), 1);
    assert_eq!(ticks[0].notch, key);
    assert_eq!(ticks[0].count, NotchCount::Double);
    let tract = tracts.iter().find(|it| it.node == from).expect("drawn");
    let ends = (tract.line[0], *tract.line.last().expect("two ends"));
    let middle = [0, 1].map(|axis| f64::midpoint(ends.0[axis], ends.1[axis]));
    // The waist runs straight across the front, so half its arc length is half
    // way between its two nodes, and the way it runs is a unit vector.
    assert!(away(ticks[0].at, middle) < 1.0e-9, "{:?}", ticks[0]);
    let length = away([0.0, 0.0], ticks[0].along);
    assert!((length - 1.0).abs() < 1.0e-9, "{length}");
}

/// A notch on a tract the contour no longer runs through is not drawn, and
/// draws nothing rather than putting a mark at the origin.
#[test]
fn a_notch_anchored_nowhere_is_not_drawn() {
    let mut doc = block::trouser_front();
    let piece = doc.piece_named(block::FRONT).expect("the block draws one");
    doc.notches.insert(Notch::lone(EdgeAnchor::at_node(
        piece,
        PointKey::new(999, 0),
    )));
    let table = table_of(doc);
    let tracts = tract::of(&table.draft, table.piece);
    assert!(ticks(&table.draft, table.piece, &tracts).is_empty());
}

#[test]
fn rubbing_a_line_out_is_one_command_and_one_entry() {
    let line = LineKey::new(3, 0);
    let (gesture, commands, feedback) = erased(line);
    assert_eq!(gesture, Gesture::Idle);
    assert_eq!(commands, vec![Command::RemoveLine { line }]);
    assert_eq!(feedback.stack, Some(Stack::Once(ERASE)));
    assert_eq!(feedback.select, Some(Selection::None));
}

/// A span that bends is drawn as the curve it was written as, and the count of
/// places stays the count of presses rather than of flattened points.
#[test]
fn a_bending_span_is_flattened_and_still_counts_as_one_step() {
    let (mut doc, _) = drawn(LineKind::Stitch, ["cintura_cf", "cadera_lat"]);
    let piece = doc.piece_named(block::FRONT).expect("the block draws one");
    let node = |label: &str| doc.shows_label(piece, label).expect("named");
    let place = |label| LineVertex::Contour(EdgeAnchor::at_node(piece, node(label)));
    let bow = SegmentEdit::cubic(Point::at(20.0, -8.0), Point::at(30.0, 8.0));
    let edit = LineEdit::new(piece, LineKind::Slit, place("cintura_cf")).curving(
        place("cadera_lat"),
        bow,
        SAMPLES,
    );
    Command::AddLine {
        identity: Identity::New,
        line: Box::new(edit),
    }
    .apply(&mut doc)
    .expect("both places are nodes of the front");

    let table = table_of(doc);
    let bent = &table.drawn()[1];
    assert_eq!(bent.places, 2, "two presses, however it bends");
    assert_eq!(
        bent.run.len(),
        usize::from(SAMPLES) + 1,
        "the samples the document asked for, and the place it ends on"
    );
    let straight = &table.drawn()[0].run;
    let chord = away(bent.run[0], *bent.run.last().expect("two ends"));
    let along: f64 = bent.run.windows(2).map(|pair| away(pair[0], pair[1])).sum();
    assert!(
        along > chord + 1.0,
        "it bows off its chord: {along} {chord}"
    );
    assert_eq!(straight.len(), 2, "and the straight one is still two ends");
}
