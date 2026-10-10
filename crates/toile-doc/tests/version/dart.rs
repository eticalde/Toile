use toile_doc::{
    Command, Dart, DartWedge, Doc, FORMAT_VERSION_DARTED, FoldDirection, History, Identity, Point,
    PointKey, SeamKey, WedgeNode, block,
};

use super::elastic::elasticated;
use super::hang::hung;
use super::line::lined;
use super::placement::placed;
use super::symmetry::folded;
use super::{EMPTY, SHIPPED, header, linked, restamped, rewritten, shaped};

/// The edit that cuts a waist dart into the back of the shipped block.
fn cut(doc: &Doc) -> Command {
    let piece = doc.piece_named(block::BACK).expect("the block draws one");
    let after = doc
        .shows_label(piece, "cintura_cb")
        .expect("the block names it");
    Command::AddDart {
        identity: Identity::New,
        dart: Dart {
            apex: PointKey::new(0, 0),
            legs: (PointKey::new(0, 0), PointKey::new(0, 0)),
            seam: SeamKey::new(0, 0),
            fold: FoldDirection::TowardStart,
        },
        wedge: Box::new(DartWedge {
            piece,
            after: Some(after),
            nodes: [
                WedgeNode::line(Identity::New, Point::at(51.0, 0.0)),
                WedgeNode::line(Identity::New, Point::at(52.0, 12.0)),
                WedgeNode::line(Identity::New, Point::at(53.0, 0.0)),
            ],
        }),
    }
}

/// The shipped block with a dart in its back, stamped `version`.
pub(super) fn darted(version: u32) -> String {
    let mut doc = block::trousers();
    cut(&doc)
        .apply(&mut doc)
        .expect("the node is on the contour");
    restamped(&doc.to_canonical_json(), version)
}

/// Every file on disk was written before a dart could be cut, and reading one
/// in a build that cuts them may not change a byte of it, the stamp least of
/// all.
///
/// The store itself is nothing new: a document has written its darts, empty or
/// not, since the first version there was. That is what makes this hold for the
/// real fixture rather than only for a document built in a test.
#[test]
fn every_file_written_before_the_dart_re_saves_byte_identical() {
    for file in [
        SHIPPED.to_owned(),
        EMPTY.to_owned(),
        shaped(2),
        linked(3, false),
        placed(SHIPPED, 4),
        elasticated(5),
        lined(6),
        folded(7),
        hung(8),
    ] {
        assert!(
            file.contains("\"darts\""),
            "the store predates the tool: {file}"
        );
        assert!(!file.contains("\"apex\""), "and carries none: {file}");
        assert_eq!(rewritten(&file).as_bytes(), file.as_bytes());
    }
}

/// A dart takes the number after the hang's. Builds that read version 8 carry
/// the record through a save untouched — the wedge is ordinary contour nodes
/// and the seam that closes it an ordinary seam — but none of them can read
/// those three nodes as a wedge, so stamped 8 a product would open there and be
/// edited into a dart whose record no longer describes the contour it names.
#[test]
fn the_version_stamp_is_9_exactly_when_a_contour_carries_a_dart() {
    for (file, stamp) in [
        (darted(1), 9),
        (darted(8), 9),
        (darted(9), 9),
        (restamped(SHIPPED, 9), 1),
        (restamped(&shaped(2), 9), 2),
        (restamped(&linked(3, false), 9), 3),
        (restamped(&placed(SHIPPED, 4), 9), 4),
        (restamped(&elasticated(5), 9), 5),
        (restamped(&lined(6), 9), 6),
        (restamped(&folded(7), 9), 7),
        (restamped(&hung(8), 9), 8),
    ] {
        let doc = Doc::from_json(&file).unwrap_or_else(|error| panic!("{error}: {file}"));
        assert_eq!(doc.format_version(), stamp, "{file}");
        assert!(doc.to_canonical_json().starts_with(&header(stamp)));
    }
}

#[test]
fn a_version_9_document_round_trips_byte_identical() {
    let file = darted(9);
    assert!(file.contains("\"fold\": \"toward_start\""), "{file}");
    assert_eq!(rewritten(&file).as_bytes(), file.as_bytes());
    let doc = Doc::from_json(&file).expect("this build reads version 9");
    assert_eq!(Doc::from_json(&doc.to_canonical_json()), Ok(doc));
}

/// And a version 8 document still reads, which is the other half of extending
/// the format rather than migrating it.
#[test]
fn a_version_8_document_still_reads_and_carries_no_dart() {
    let file = restamped(SHIPPED, 8);
    let doc = Doc::from_json(&file).expect("this build still reads version 8");
    assert!(doc.darts.is_empty());
    assert_eq!(doc, block::trousers());
}

/// A dart that cites a point the file does not carry is refused at the door
/// like any other key that leads nowhere: the wedge is what says those three
/// nodes are a dart, and a record naming a node that has gone describes no
/// wedge.
#[test]
fn a_dart_citing_a_node_the_file_lost_is_refused() {
    let file = darted(9);
    let edited = file.replacen("\"apex\": \"27.0\"", "\"apex\": \"77.0\"", 1);
    assert_ne!(edited, file, "the fixture moved under the test");
    let error = Doc::from_json(&edited).expect_err("the apex names no point");
    assert!(error.to_string().contains("does not carry"), "{error}");
}

/// The stamp follows the content through an undo as well: a product whose only
/// dart has been closed back up asks for the version it asked for before.
#[test]
fn a_dart_cut_and_undone_takes_the_file_to_version_9_and_back() {
    let mut doc = block::trousers();
    let mut history = History::new();
    let edit = cut(&doc);
    history
        .edit(&mut doc, edit)
        .expect("the node is on the contour");
    assert_eq!(doc.format_version(), FORMAT_VERSION_DARTED);
    let file = doc.to_canonical_json();

    history.undo(&mut doc).expect("the dart is live");
    assert_eq!(doc.format_version(), 1, "the stamp is the content's");
    history.redo(&mut doc).expect("every slot is open again");
    assert_eq!(doc.to_canonical_json(), file);
}
