use toile_doc::{
    Coalesced, Command, DocError, History, Identity, LineKey, LineKind, Point, SegmentEdit, block,
};

use super::{draw, drawn, front, mouth, node};

/// The gesture a person makes while typing a line's name.
const TYPING: &str = "nombrar línea";

/// Typing a name emits an edit per keystroke, and the gesture keeps one; the
/// kind is another field, and a second line is another line.
#[test]
fn naming_a_line_folds_into_one_edit_and_the_kind_is_its_own_field() {
    let mut doc = block::trouser_front();
    let edit = mouth(&mut doc);
    let key = draw(&mut doc, edit);
    let label = |to: &str| Command::LabelLine {
        line: key,
        to: Some(to.to_owned()),
    };
    let kind = |to| Command::SetLineKind { line: key, to };

    let mut history = History::new();
    history.begin(TYPING);
    for frame in ["ranu", "ranura", "ranura pill"] {
        history
            .edit(&mut doc, label(frame))
            .expect("the line is live");
    }
    history.end();
    assert_eq!(history.depth(), 1);
    history.undo(&mut doc).expect("the line is live");
    assert_eq!(drawn(&doc, key).label.as_deref(), Some("ranura pill"));

    assert_eq!(
        label("otra").coalesce_onto(&label("otr")),
        Coalesced::Replaces
    );
    assert_eq!(
        kind(LineKind::Fold).coalesce_onto(&kind(LineKind::Stitch)),
        Coalesced::Replaces
    );
    assert_eq!(
        kind(LineKind::Fold).coalesce_onto(&label("otra")),
        Coalesced::Separate
    );
    let other = Command::SetLineKind {
        line: LineKey::new(9, 0),
        to: LineKind::Fold,
    };
    assert_eq!(
        other.coalesce_onto(&kind(LineKind::Fold)),
        Coalesced::Separate
    );
}

#[test]
fn what_a_line_is_for_is_written_and_taken_back() {
    let mut doc = block::trouser_front();
    let edit = mouth(&mut doc);
    let key = draw(&mut doc, edit);
    let applied = Command::SetLineKind {
        line: key,
        to: LineKind::Stitch,
    }
    .apply(&mut doc)
    .expect("the line is live");
    assert_eq!(drawn(&doc, key).kind, LineKind::Stitch);
    applied.inverse.apply(&mut doc).expect("the line is live");
    assert_eq!(drawn(&doc, key).kind, LineKind::Slit);
    assert_eq!(
        Command::SetLineKind {
            line: LineKey::new(9, 0),
            to: LineKind::Fold,
        }
        .apply(&mut doc),
        Err(DocError::stale(LineKey::new(9, 0)))
    );
}

/// The ends are node keys with a fraction local to the tract leaving them, so
/// a node put inside the run is cloth the line now crosses, and not a reason
/// for either end to move.
#[test]
fn a_node_inserted_under_the_run_leaves_both_ends_where_they_were() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let edit = mouth(&mut doc);
    let key = draw(&mut doc, edit);
    let before = drawn(&doc, key);

    Command::InsertNode {
        piece,
        after: Some(node(&doc, "cintura_lat")),
        identity: Identity::New,
        value: Point::at(30.0, 5.0),
        segment: SegmentEdit::Line,
        samples: 1,
    }
    .apply(&mut doc)
    .expect("the contour runs through the waist");
    assert_eq!(drawn(&doc, key), before);
}

/// The same as a seam and an elastic: the line waits under its own key, the
/// arena never hands the slot to anything else, and the undo gives the piece
/// back with everything that was drawn on it.
#[test]
fn a_line_on_a_piece_taken_off_the_table_comes_back_with_it() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let edit = mouth(&mut doc);
    draw(&mut doc, edit);
    let before = doc.clone();
    let mut history = History::new();

    history
        .edit(&mut doc, Command::RemovePiece { piece })
        .expect("the piece is live");
    assert_eq!(doc.lines.len(), 1, "the line waits for its piece");
    history.undo(&mut doc).expect("the slot is free again");
    assert_eq!(doc, before);
}
