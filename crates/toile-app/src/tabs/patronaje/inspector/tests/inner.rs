use toile_engine::draft::{LineKey, LineKind};

use super::super::inner::{erase_id, kind_id, row_id};
use super::desk::{Desk, bytes, piece_open};
use crate::tabs::patronaje::inner::tests::drawn;
use crate::tabs::patronaje::state::Selection;

/// The block with one fold drawn across its front, on a tall desk, open on that
/// piece with nothing chosen.
fn folded() -> (Desk, LineKey) {
    let (doc, line) = drawn(LineKind::Fold, ["cintura_cf", "cadera_lat"]);
    let (desk, _) = piece_open(doc);
    (desk, line)
}

/// What the document says one line is for.
fn kind_of(desk: &Desk, line: LineKey) -> LineKind {
    desk.session
        .draft()
        .expect("a product is open")
        .doc()
        .lines
        .get(line)
        .expect("the line is live")
        .kind
}

/// Looking at the lines of a piece is not editing them: reading the list,
/// choosing a line and letting it go all leave the file the file that was
/// opened.
#[test]
fn looking_at_the_lines_of_a_piece_writes_nothing() {
    let (mut desk, line) = folded();
    let opened = bytes(&desk);
    assert!(desk.drew(row_id(line)), "the line is listed");
    assert_eq!(desk.session.revision(), 0, "nothing reached the document");

    desk.click(desk.centre(row_id(line)));
    desk.frame(Vec::new());
    assert_eq!(desk.state.selection, Selection::Line(line));
    assert!(
        desk.drew(kind_id(line)),
        "and its controls take their place"
    );
    assert!(desk.drew(erase_id(line)));

    desk.click(desk.centre(row_id(line)));
    desk.frame(Vec::new());
    assert_eq!(
        desk.state.selection,
        Selection::None,
        "a second press lets go"
    );
    assert!(!desk.drew(kind_id(line)), "and the controls go with it");
    assert_eq!(desk.entries(), 0, "nothing reached the history");
    assert_eq!(bytes(&desk), opened, "the bytes are the bytes it opened");
}

/// One press steps what the line is for, as one entry of its own, and the line
/// keeps its key: what a fold is for is not a different line.
#[test]
fn stepping_the_kind_is_one_entry_and_the_line_keeps_its_key() {
    let (mut desk, line) = folded();
    desk.click(desk.centre(row_id(line)));
    desk.frame(Vec::new());
    assert_eq!(kind_of(&desk, line), LineKind::Fold);

    desk.click(desk.centre(kind_id(line)));
    desk.frame(Vec::new());
    assert_eq!(kind_of(&desk, line), LineKind::Stitch, "one step on");
    assert_eq!(desk.entries(), 1, "one press, one entry");
    assert_eq!(desk.session.undo_label(), Some("cambiar el tipo de línea"));
    assert_eq!(desk.state.selection, Selection::Line(line), "still chosen");

    desk.session.undo().expect("the entry steps back");
    assert_eq!(kind_of(&desk, line), LineKind::Fold);
}

/// The press rubs the line out in one entry, and undo draws the same line again
/// under its own key, with what it was for.
#[test]
fn the_line_comes_off_in_one_entry_and_undo_draws_it_again() {
    let (mut desk, line) = folded();
    desk.click(desk.centre(row_id(line)));
    desk.frame(Vec::new());

    desk.click(desk.centre(erase_id(line)));
    desk.frame(Vec::new());
    let doc = desk.session.draft().expect("a product is open").doc();
    assert_eq!(doc.lines.len(), 0, "the piece is bare again");
    assert_eq!(desk.entries(), 1, "one press, one entry");
    assert_eq!(desk.session.undo_label(), Some("borrar línea"));
    // Twice: egui answers for a widget against the pass that drew it as well as
    // the one before, so one frame after the removal the row is still on
    // record.
    desk.frame(Vec::new());
    desk.frame(Vec::new());
    assert!(!desk.drew(row_id(line)), "and nothing is left to list");

    desk.session.undo().expect("the removal steps back");
    assert_eq!(kind_of(&desk, line), LineKind::Fold, "key and all");
}

/// The row of the section and its two controls stack instead of landing on one
/// another: a control drawn over another is a control nobody can aim at.
#[test]
fn the_row_and_its_two_controls_do_not_land_on_one_another() {
    let (mut desk, line) = folded();
    desk.click(desk.centre(row_id(line)));
    desk.frame(Vec::new());
    let (row, kind, erase) = (
        desk.rect(row_id(line)),
        desk.rect(kind_id(line)),
        desk.rect(erase_id(line)),
    );
    assert!(row.bottom() < kind.top(), "{row:?} {kind:?}");
    assert!(kind.bottom() < erase.top(), "{kind:?} {erase:?}");
    assert!(erase.right() < 1320.0, "inside the panel: {erase:?}");
}
