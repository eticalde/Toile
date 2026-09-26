use toile_engine::draft::{
    Command, Dart, DartKey, DartWedge, Doc, EdgeAnchor, FoldDirection, Identity, Notch, Point,
    SeamKey, WedgeNode, block,
};

use super::super::dart::{loosen_id, take_off_id};
use super::desk::{Desk, bytes, piece_open};

/// The shipped front with one dart cut into its waistline, open on the piece.
///
/// The waistline because that is the run a dart is put on: its own wedge is
/// straight, four centimetres wide and six deep, so the numbers the panel reads
/// off it are numbers this test can check by hand.
fn darted() -> (Desk, DartKey) {
    opened(with_dart())
}

/// The same front, with a mark pinned to the tract leaving the wedge's first
/// leg.
fn marked() -> (Desk, DartKey) {
    let mut doc = with_dart();
    let piece = doc.piece_keys()[0];
    let leg = doc
        .darts
        .iter()
        .map(|(_, held)| held.legs.0)
        .next()
        .expect("the cut wrote one");
    Command::AddNotch {
        identity: Identity::New,
        notch: Notch::lone(EdgeAnchor {
            piece,
            from: leg,
            t: 0.5,
        }),
        mate: None,
    }
    .apply(&mut doc)
    .expect("a mark goes on the tract");
    opened(doc)
}

/// The desk with that product on it, and the key of the dart it carries.
fn opened(doc: Doc) -> (Desk, DartKey) {
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

/// The shipped front with one dart cut into its waistline.
fn with_dart() -> Doc {
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
    doc
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
    assert_eq!(desk.session.undo_label(), Some("quitar la pinza y su cuña"));

    desk.session.undo().expect("the removal steps back");
    assert_eq!(bytes(&desk), before, "the same dart, under its own keys");
}

/// Letting the dart go takes the record and the thread and leaves the wedge.
///
/// The record cannot say whether a dart was cut or declared — that is what kept
/// the format at one version — so the panel cannot know either, and it offers
/// both presses instead of guessing. This is the one a drafted wedge needs: the
/// three nodes stay in the contour with the coordinates they were drawn with,
/// so changing which way a dart is pressed no longer costs a person their
/// formulas.
#[test]
fn letting_the_dart_go_keeps_its_wedge_and_takes_only_the_thread() {
    let (mut desk, key) = darted();
    let doc = desk.session.draft().expect("a product is open").doc();
    let piece = doc.piece_keys()[0];
    let contour = doc
        .pieces
        .get(piece)
        .expect("the key is live")
        .contour
        .clone();
    let points = doc.points.len();
    assert!(desk.drew(loosen_id(key)), "it offers to let the dart go");

    desk.click(desk.centre(loosen_id(key)));
    desk.frame(Vec::new());
    let doc = desk.session.draft().expect("a product is open").doc();
    assert_eq!(doc.darts.len(), 0, "the record is gone");
    assert_eq!(doc.seams.len(), 0, "and the thread that shut it");
    assert_eq!(doc.points.len(), points, "and not one node went with them");
    assert_eq!(
        doc.pieces.get(piece).expect("the key is live").contour,
        contour,
        "the wedge is still drawn, node for node"
    );
    assert_eq!(desk.entries(), 1, "one press, one entry");
    assert_eq!(desk.session.undo_label(), Some("dejar de llamarla pinza"));
}

/// A wedge something else of the pattern is pinned to is not offered for
/// removal, and is still offered the door that keeps it.
///
/// The destructive press takes the three nodes out of the document, and a
/// product whose mark names a node it no longer carries never opens again —
/// measured at 82,408 bytes before the document refused it. So the panel asks
/// the document the same question the removal asks, and where the answer is yes
/// it offers one press instead of two and says why.
#[test]
fn a_wedge_a_mark_is_pinned_to_is_not_offered_for_removal() {
    let (mut desk, key) = marked();
    desk.frame(Vec::new());
    assert!(
        desk.drew(loosen_id(key)),
        "the door that keeps the wedge stays open"
    );
    assert!(
        !desk.drew(take_off_id(key)),
        "and the one that would take the mark's own node with it is not offered"
    );
    assert_eq!(desk.session.revision(), 0, "nothing reached the document");
}

/// Letting a wedge go that a mark is pinned to leaves a product that opens.
///
/// The one door left has to lead somewhere: it takes the record and the thread,
/// leaves every node, and the mark on the leg is still pinned to a node the
/// file carries.
#[test]
fn the_wedge_under_a_mark_can_still_stop_being_a_dart() {
    let (mut desk, key) = marked();
    desk.click(desk.centre(loosen_id(key)));
    desk.frame(Vec::new());

    let doc = desk.session.draft().expect("a product is open").doc();
    assert_eq!(doc.darts.len(), 0, "the record is gone");
    assert_eq!(doc.notches.len(), 1, "and the mark is where it was");
    Doc::from_json(&bytes(&desk)).expect("the product opens");
}
