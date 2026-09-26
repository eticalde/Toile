use toile_doc::{
    Command, DartKey, Doc, DocError, DrawnWedge, Identity, PieceKey, PointKey, Seam,
    SeamOrientation, block,
};

use super::{BESIDE, LEVEL, asked, contour, declare, drafted, drawn_in, only_dart};

/// Three contiguous nodes of the back with two of them bound to one place.
fn level(doc: &mut Doc, piece: PieceKey) -> [PointKey; 3] {
    after_the_draft(doc, piece, LEVEL)
}

/// A second wedge drawn beside the first, as good a wedge as the first is.
fn beside(doc: &mut Doc, piece: PieceKey) -> [PointKey; 3] {
    after_the_draft(doc, piece, BESIDE)
}

/// Three nodes written in after the wedge the draft already carries.
fn after_the_draft(
    doc: &mut Doc,
    piece: PieceKey,
    write: [(&str, &str, &str); 3],
) -> [PointKey; 3] {
    let after = doc
        .shows_label(piece, "pinza_b")
        .expect("the draft named it");
    drawn_in(doc, piece, after, write)
}

/// One of the contour's handles, which is a live point no node of it is.
fn handle(doc: &Doc, piece: PieceKey) -> PointKey {
    contour(doc, piece)
        .iter()
        .find_map(|node| node.segment.handles())
        .expect("the block bends the back hip")
        .0
}

/// Every refusal the declaration makes, each of them before anything is
/// written.
///
/// Of the five the review named, two are one rule here and two more are a
/// second. Three nodes out of contour order and a middle node that is not
/// between the other two are the same rule — the three stand together, leg,
/// apex and leg — so they share one error. Three nodes on different pieces is a
/// node this contour does not have, because the wedge names one piece for all
/// three and cannot say otherwise; that is the same refusal as a node that is
/// not on the contour at all. `FlatWedge` is the rule the cut already owns,
/// reused and not doubled: two nodes bound to one place leave a seam side with
/// no length.
#[test]
fn a_declaration_no_contour_could_carry_writes_nothing() {
    let (mut doc, piece, nodes) = drafted();
    let front = doc.piece_named(block::FRONT).expect("the block draws one");
    let elsewhere = doc
        .shows_label(front, "cintura_cf")
        .expect("the block names it");
    let side = doc
        .shows_label(piece, "cintura_lat_tras")
        .expect("the block names it");
    let handle = handle(&doc, piece);
    let flat = level(&mut doc, piece);
    let before = doc.clone();
    let on = |nodes: [PointKey; 3]| DrawnWedge { piece, nodes };
    let cases = [
        (
            DrawnWedge {
                piece: PieceKey::new(9, 0),
                nodes,
            },
            DocError::stale(PieceKey::new(9, 0)),
        ),
        (on([nodes[0], nodes[1], elsewhere]), DocError::NoSuchNode),
        (on([nodes[0], handle, nodes[2]]), DocError::NoSuchNode),
        (on([nodes[0], nodes[1], side]), DocError::ScatteredWedge),
        (on([nodes[1], nodes[0], nodes[2]]), DocError::ScatteredWedge),
        (on([nodes[0], nodes[1], nodes[1]]), DocError::ScatteredWedge),
        (on(flat), DocError::FlatWedge),
    ];
    for (asking, why) in cases {
        let refused = Command::DeclareDart {
            identity: Identity::New,
            dart: asked(),
            wedge: asking,
        }
        .apply(&mut doc);
        assert_eq!(refused, Err(why));
        assert_eq!(doc, before, "nothing was written");
    }
}

/// A node one dart already names is not one another dart may name.
///
/// Two darts over one node are two seams pulling one place and two mouths
/// crossing on one printed sheet. The plainest case is the same wedge declared
/// twice, which is a press that landed twice.
#[test]
fn a_node_a_dart_already_names_is_refused_to_the_next_one() {
    let (mut doc, piece, nodes) = drafted();
    let beside = level(&mut doc, piece);
    declare(piece, nodes).apply(&mut doc).expect("it fits");
    let before = doc.clone();

    let twice = declare(piece, nodes).apply(&mut doc);
    assert_eq!(twice, Err(DocError::AlreadyDarted));
    assert_eq!(doc, before, "nothing was written");

    // And a wedge that shares only a leg: the second leg of the dart just
    // declared, with two nodes of its own standing after it.
    let sharing = DrawnWedge {
        piece,
        nodes: [nodes[2], beside[0], beside[1]],
    };
    let refused = Command::DeclareDart {
        identity: Identity::New,
        dart: asked(),
        wedge: sharing,
    }
    .apply(&mut doc);
    assert_eq!(refused, Err(DocError::AlreadyDarted));
    assert_eq!(doc, before, "nothing was written");
}

/// A declaration told to restore keys that are taken is refused, and a dart
/// that is not there cannot be taken off.
#[test]
fn a_declaration_on_taken_keys_and_a_dart_that_is_gone_are_both_refused() {
    let (mut doc, piece, nodes) = drafted();
    let apart = beside(&mut doc, piece);
    declare(piece, nodes).apply(&mut doc).expect("it fits");
    let (key, held) = only_dart(&doc);
    let before = doc.clone();

    // A wedge of its own, so that the only thing wrong is the key it asks back.
    let again = Command::DeclareDart {
        identity: Identity::Restored(key),
        dart: held,
        wedge: DrawnWedge {
            piece,
            nodes: apart,
        },
    }
    .apply(&mut doc);
    assert_eq!(again, Err(DocError::occupied(key)));
    assert_eq!(doc, before, "nothing was written");

    let gone = DartKey::new(9, 0);
    assert_eq!(
        Command::UndeclareDart { dart: gone }.apply(&mut doc),
        Err(DocError::stale(gone))
    );
    assert_eq!(doc, before, "nothing was written");
}

/// The dart's own thread is not one to pull, whichever edit reaches for it.
///
/// Unpicking it is refused, and with it every way of re-sewing it: the document
/// has no edit that rewrites a seam, so a hand that turns one round takes it
/// out and puts it back under its key, and the taking out is this refusal. A
/// dart re-sewn the other way round sews each leg to the apex and leaves the
/// wedge open with a record that still says dart.
#[test]
fn the_thread_that_shuts_a_declared_dart_cannot_be_pulled_or_turned_round() {
    let (mut doc, piece, nodes) = drafted();
    declare(piece, nodes).apply(&mut doc).expect("it fits");
    let (key, held) = only_dart(&doc);
    let sewn = *doc
        .seams
        .get(held.seam)
        .expect("the declaration issued one");
    assert_eq!(doc.dart_closed_by(held.seam), Some(key));
    let before = doc.to_canonical_json();

    let unpick = Command::RemoveSeam { seam: held.seam }
        .apply(&mut doc)
        .expect_err("the dart's own thread stays where it is");
    assert_eq!(unpick, DocError::SeamClosesADart);
    let turned = Command::AddSeam {
        identity: Identity::Restored(held.seam),
        seam: Seam {
            orientation: SeamOrientation::Aligned,
            ..sewn
        },
    }
    .apply(&mut doc)
    .expect_err("its slot is not open, because nothing took it out");
    assert_eq!(turned, DocError::occupied(held.seam));

    assert_eq!(
        doc.seams.get(held.seam).map(|it| it.orientation),
        Some(SeamOrientation::Opposed),
        "and the wedge is still shut"
    );
    assert_eq!(doc.to_canonical_json(), before, "and nothing was written");
}

/// A file the editor cannot write is a file the reader will not open.
///
/// The file is the one way in that can say a dart is shut and be wrong, so that
/// is where the rule is asked again: a record whose wedge no longer stands
/// together, and one whose seam pairs each leg with the apex instead of leg
/// with leg, are both refused by name rather than opened and draped as darts.
#[test]
fn a_file_whose_dart_does_not_describe_its_contour_is_refused_by_name() {
    let (mut doc, piece, nodes) = drafted();
    declare(piece, nodes).apply(&mut doc).expect("it fits");
    let (_, held) = only_dart(&doc);
    let written = doc.to_canonical_json();
    assert!(Doc::from_json(&written).is_ok(), "the written one opens");

    let mut turned = doc.clone();
    turned
        .seams
        .get_mut(held.seam)
        .expect("the key is live")
        .orientation = SeamOrientation::Aligned;
    let why = Doc::from_json(&turned.to_canonical_json()).expect_err("it is not shut");
    assert_eq!(
        why.to_string(),
        "the pattern carries a dart its contour does not: a dart's seam sews one leg to \
         the other through its apex, and this one does not"
    );

    let mut broken = doc.clone();
    broken
        .pieces
        .get_mut(piece)
        .expect("the key is live")
        .contour
        .swap(1, 2);
    let why = Doc::from_json(&broken.to_canonical_json()).expect_err("the wedge is scattered");
    assert_eq!(
        why.to_string(),
        "the pattern carries a dart its contour does not: a dart's wedge is three nodes \
         standing together in the contour, leg, apex and leg"
    );
}
