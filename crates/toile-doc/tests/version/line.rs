use toile_doc::{
    Command, Doc, EdgeAnchor, FORMAT_VERSION_INTERNAL, History, Identity, LineEdit, LineKind,
    VertexEdit, block,
};

use super::elastic::elasticated;
use super::placement::placed;
use super::{EMPTY, SHIPPED, header, linked, restamped, rewritten, shaped};

/// The edit that marks a fold across the waist of the shipped block's front.
fn fold(doc: &Doc) -> Command {
    let front = doc.piece_named(block::FRONT).expect("the block draws one");
    let named = |label| doc.shows_label(front, label).expect("the block names it");
    let head = VertexEdit::Contour(EdgeAnchor::at_node(front, named("cintura_cf")));
    let tail = VertexEdit::Contour(EdgeAnchor {
        piece: front,
        from: named("cintura_lat"),
        t: 0.5,
    });
    Command::AddLine {
        identity: Identity::New,
        line: Box::new(
            LineEdit::new(front, LineKind::Fold, head)
                .to(tail)
                .named("quiebre"),
        ),
    }
}

/// The shipped block with that fold drawn on it, stamped `version`.
pub(super) fn lined(version: u32) -> String {
    let mut doc = block::trousers();
    let edit = fold(&doc);
    edit.apply(&mut doc).expect("both places are on the front");
    restamped(&doc.to_canonical_json(), version)
}

/// Every file on disk was written before a piece could be drawn with a line it
/// is not cut on, and reading one in a build that knows the internal line may
/// not change a byte of it, the stamp least of all.
#[test]
fn every_file_written_before_the_internal_line_re_saves_byte_identical() {
    for file in [
        SHIPPED.to_owned(),
        EMPTY.to_owned(),
        shaped(2),
        linked(3, false),
        linked(3, true),
        placed(SHIPPED, 4),
        elasticated(5),
    ] {
        assert!(!file.contains("\"lines\""), "{file}");
        assert_eq!(rewritten(&file).as_bytes(), file.as_bytes());
    }
}

/// An internal line takes the number after the elastic's. Builds that read
/// version 5 predate it: stamped 5, a product would open there with every fold,
/// topstitch, pocket mouth and buttonhole gone, and saving would make that for
/// good — a pattern nobody can cut from paper, with nothing on screen to say
/// what was lost.
#[test]
fn the_version_stamp_is_6_exactly_when_the_document_draws_an_internal_line() {
    for (file, stamp) in [
        (lined(1), 6),
        (lined(5), 6),
        (lined(6), 6),
        (restamped(SHIPPED, 6), 1),
        (restamped(&shaped(2), 6), 2),
        (restamped(&linked(3, false), 6), 3),
        (restamped(&placed(SHIPPED, 4), 6), 4),
        (restamped(&elasticated(5), 6), 5),
    ] {
        let doc = Doc::from_json(&file).unwrap_or_else(|error| panic!("{error}: {file}"));
        assert_eq!(doc.format_version(), stamp, "{file}");
        assert!(doc.to_canonical_json().starts_with(&header(stamp)));
    }
}

#[test]
fn a_version_6_document_round_trips_byte_identical() {
    let file = lined(6);
    assert!(file.contains("\"kind\": \"fold\","), "{file}");
    assert!(file.contains("\"label\": \"quiebre\","), "{file}");
    assert!(file.contains("\"kind\": \"contour\","), "{file}");
    assert!(file.contains("\"samples\": 1"), "{file}");
    assert_eq!(rewritten(&file).as_bytes(), file.as_bytes());
    let doc = Doc::from_json(&file).expect("this build reads version 6");
    assert_eq!(Doc::from_json(&doc.to_canonical_json()), Ok(doc));
}

/// The stamp follows the content through an undo as well: a product whose only
/// line has been rubbed out again is the file it was before, stamp and all.
#[test]
fn a_line_drawn_and_undone_takes_the_file_to_version_6_and_back() {
    let mut doc = block::trousers();
    let before = doc.to_canonical_json();
    let mut history = History::new();
    let edit = fold(&doc);
    history
        .edit(&mut doc, edit)
        .expect("both places are on the front");
    assert_eq!(doc.format_version(), FORMAT_VERSION_INTERNAL);
    let file = doc.to_canonical_json();

    history.undo(&mut doc).expect("the line is live");
    assert_eq!(
        doc.to_canonical_json(),
        before,
        "byte for byte, stamp first"
    );
    history.redo(&mut doc).expect("the slot is free");
    assert_eq!(doc.to_canonical_json(), file);
}

/// A version-7 file carries a fold, which this build reads; the refusal a
/// later stamp earns is the fold's own case, next door.
#[test]
fn a_version_7_stamp_on_a_file_with_no_axis_falls_back_to_what_it_carries() {
    for (file, stamp) in [(restamped(&lined(6), 7), 6), (restamped(SHIPPED, 7), 1)] {
        let doc = Doc::from_json(&file).expect("this build reads version 7");
        assert_eq!(doc.format_version(), stamp, "{file}");
    }
}
