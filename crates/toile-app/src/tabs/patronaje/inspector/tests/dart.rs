use toile_engine::draft::{
    Command, Dart, DartKey, DartWedge, FoldDirection, Identity, Point, SeamKey, WedgeNode, block,
};

use super::super::dart::take_off_id;
use super::desk::{Desk, bytes, piece_open};

/// The shipped front with one dart cut into its waistline, open on the piece.
///
/// The waistline because that is the run a dart is put on: its own wedge is
/// straight, four centimetres wide and six deep, so the numbers the panel reads
/// off it are numbers this test can check by hand.
fn darted() -> (Desk, DartKey) {
    let mut doc = block::trouser_front();
    let piece = doc
        .piece_named(block::FRONT)
        .expect("the block draws one piece");
    let node = doc
        .shows_label(piece, "cintura_cf")
        .expect("the block names the waist at the centre front");
    let at = |x: f64, y: f64| WedgeNode::line(Identity::New, Point::at(x, y));
    // Every key the dart names is one the edit itself issues, so only the fold
    // is read off here.
    Command::AddDart {
        identity: Identity::New,
        dart: Dart {
            apex: node,
            legs: (node, node),
            seam: SeamKey::new(0, 0),
            fold: FoldDirection::TowardStart,
        },
        wedge: Box::new(DartWedge {
            piece,
            after: Some(node),
            nodes: [at(8.0, 0.0), at(10.0, 6.0), at(12.0, 0.0)],
        }),
    }
    .apply(&mut doc)
    .expect("the wedge sits on the waistline");
    let (desk, _) = piece_open(doc);
    let key = desk
        .session
        .draft()
        .expect("a product is open")
        .doc()
        .darts
        .iter()
        .map(|(key, _)| key)
        .next()
        .expect("the cut wrote one");
    (desk, key)
}

/// Reading a dart is not editing it.
///
/// The section lays the dart out and offers the press that takes it off; until
/// that press the file is the file that was opened, byte for byte.
#[test]
fn a_dart_on_the_piece_is_laid_out_and_writes_nothing() {
    let (mut desk, key) = darted();
    let opened = bytes(&desk);
    desk.frame(Vec::new());
    assert!(
        desk.drew(take_off_id(key)),
        "it offers to take the dart off"
    );
    assert_eq!(desk.session.revision(), 0, "nothing reached the document");
    assert_eq!(desk.entries(), 0, "and nothing reached the history");
    assert_eq!(bytes(&desk), opened, "the bytes are the bytes it opened");
}

/// Taking it off is one entry, and undo cuts the same dart again under its own
/// key, with its wedge and its seam back with it.
#[test]
fn the_dart_comes_off_in_one_entry_and_undo_cuts_it_again() {
    let (mut desk, key) = darted();
    let before = bytes(&desk);
    desk.click(desk.centre(take_off_id(key)));
    desk.frame(Vec::new());

    let doc = desk.session.draft().expect("a product is open").doc();
    assert_eq!(doc.darts.len(), 0, "the contour is whole again");
    assert_eq!(doc.seams.len(), 0, "and the seam that closed it is gone");
    assert_eq!(desk.entries(), 1, "one press, one entry");
    assert_eq!(desk.session.undo_label(), Some("quitar la pinza"));

    desk.session.undo().expect("the removal steps back");
    assert_eq!(bytes(&desk), before, "the same dart, under its own keys");
}
