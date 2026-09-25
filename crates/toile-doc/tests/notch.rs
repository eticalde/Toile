#![allow(missing_docs, reason = "a test crate publishes no API surface")]
#![allow(
    clippy::float_cmp,
    reason = "a notch keeps the fraction it was written with"
)]

use toile_doc::{
    ChangeClass, Coalesced, Command, Doc, DocError, EdgeAnchor, History, Identity, Notch, NotchKey,
    PieceKey, PointKey, block,
};

/// The name the gesture that slides a mark carries into the status bar.
const SLIDE: &str = "mover piquete";

/// The block's front, one of its named nodes, and a mark cut half way along the
/// tract leaving that node.
fn marked() -> (Doc, PieceKey, PointKey, NotchKey) {
    let mut doc = block::trousers();
    let piece = doc.piece_named(block::FRONT).expect("the block draws one");
    let from = doc
        .shows_label(piece, "cintura_cf")
        .expect("the block names it");
    let notch = cut(&mut doc, at(piece, from, 0.5), None);
    (doc, piece, from, notch)
}

fn at(piece: PieceKey, from: PointKey, t: f64) -> EdgeAnchor {
    EdgeAnchor { piece, from, t }
}

fn cut(doc: &mut Doc, place: EdgeAnchor, mate: Option<EdgeAnchor>) -> NotchKey {
    let applied = Command::AddNotch {
        identity: Identity::New,
        notch: Notch::lone(place),
        mate: mate.map(|held| (Identity::New, Notch::lone(held))),
    }
    .apply(doc)
    .expect("the places are on the contour");
    let Command::RemoveNotch { notch } = applied.inverse else {
        panic!("the inverse takes the mark off the contour");
    };
    notch
}

fn place(doc: &Doc, notch: NotchKey) -> EdgeAnchor {
    doc.notches.get(notch).expect("the mark is live").at
}

/// The mark slides along its tract and the inverse puts it back where it was.
#[test]
fn a_mark_slides_and_the_inverse_puts_it_back() {
    let (mut doc, piece, from, notch) = marked();
    let command = Command::MoveNotch {
        notch,
        to: at(piece, from, 0.25),
    };
    assert_eq!(command.class(), ChangeClass::Topology);
    let applied = command.apply(&mut doc).expect("the mark is live");
    assert_eq!(place(&doc, notch).t, 0.25);
    assert_eq!(applied.touched, vec![piece], "one piece, named once");
    assert_eq!(applied.class, ChangeClass::Topology);

    applied.inverse.apply(&mut doc).expect("the mark is live");
    assert_eq!(place(&doc, notch).t, 0.5, "back where it was cut");
}

/// The place comes through the one door every place on a contour comes through:
/// a fraction off the end of its tract, a point no contour runs through, and a
/// piece the document has lost are all refused, and nothing moves.
#[test]
fn a_place_no_contour_answers_for_is_refused_and_the_mark_stays() {
    let (mut doc, piece, from, notch) = marked();
    let handle = doc
        .pieces
        .get(piece)
        .and_then(|held| held.contour.iter().find_map(|node| node.segment.handles()))
        .map(|(out, _)| out)
        .expect("the front bends two tracts");
    let stray = PointKey::new(90, 0);
    for (to, expected) in [
        (at(piece, from, 1.5), DocError::AnchorFraction),
        (at(piece, from, f64::NAN), DocError::AnchorFraction),
        (at(piece, handle, 0.5), DocError::NoSuchNode),
        (at(piece, stray, 0.5), DocError::stale(stray)),
        (
            at(PieceKey::new(9, 0), from, 0.5),
            DocError::stale(PieceKey::new(9, 0)),
        ),
    ] {
        let refused = Command::MoveNotch { notch, to }.apply(&mut doc);
        assert_eq!(refused, Err(expected), "{to:?}");
        assert_eq!(place(&doc, notch).t, 0.5, "the mark did not move");
    }
    let gone = Command::MoveNotch {
        notch: NotchKey::new(9, 0),
        to: at(piece, from, 0.25),
    }
    .apply(&mut doc);
    assert_eq!(gone, Err(DocError::stale(NotchKey::new(9, 0))));
}

/// One drag is one entry: a frame per pointer move folds onto the one before,
/// and undo goes back to where the mark stood before the gesture rather than to
/// the last frame of it.
#[test]
fn a_drag_of_a_mark_folds_into_one_entry() {
    let (mut doc, piece, from, notch) = marked();
    let mut history = History::new();
    history.begin(SLIDE);
    for t in [0.4, 0.3, 0.2] {
        history
            .edit(
                &mut doc,
                Command::MoveNotch {
                    notch,
                    to: at(piece, from, t),
                },
            )
            .expect("the mark is live");
    }
    history.end();
    assert_eq!(history.depth(), 1, "one gesture, one entry");
    assert_eq!(place(&doc, notch).t, 0.2);
    history.undo(&mut doc).expect("the mark is live");
    assert_eq!(place(&doc, notch).t, 0.5, "and not the frame before");

    let slid = |t| Command::MoveNotch {
        notch,
        to: at(piece, from, t),
    };
    assert_eq!(slid(0.3).coalesce_onto(&slid(0.4)), Coalesced::Replaces);
    let other = Command::MoveNotch {
        notch: NotchKey::new(9, 0),
        to: at(piece, from, 0.3),
    };
    assert_eq!(other.coalesce_onto(&slid(0.4)), Coalesced::Separate);
}

/// A mark sewn to another does not drag its twin along: where the facing mark
/// sits is a question about the facing contour, and moving it unasked would
/// undo a placement somebody made.
#[test]
fn sliding_one_of_a_pair_leaves_its_twin_where_it_was() {
    let mut doc = block::trousers();
    let piece = doc.piece_named(block::FRONT).expect("the block draws one");
    let back = doc.piece_named(block::BACK).expect("the block draws two");
    let named = |key, label: &str| {
        doc.shows_label(key, label)
            .unwrap_or_else(|| panic!("the block names {label}"))
    };
    let (front_node, back_node) = (named(piece, "cintura_cf"), named(back, "cintura_cb"));
    let notch = cut(
        &mut doc,
        at(piece, front_node, 0.5),
        Some(at(back, back_node, 0.5)),
    );
    let twin = doc
        .notches
        .get(notch)
        .and_then(|held| held.mate)
        .expect("the pair names itself both ways");
    Command::MoveNotch {
        notch,
        to: at(piece, front_node, 0.1),
    }
    .apply(&mut doc)
    .expect("the mark is live");
    assert_eq!(place(&doc, twin).t, 0.5, "the twin stayed");
    assert_eq!(place(&doc, twin).piece, back);
    assert_eq!(
        doc.notches.get(notch).and_then(|held| held.mate),
        Some(twin),
        "and the two still name each other"
    );
}
