use super::*;

/// A mark pinned to one of the wedge's own nodes, on the tract that leaves it.
fn mark(doc: &mut Doc, piece: PieceKey, from: PointKey) {
    Command::AddNotch {
        identity: Identity::New,
        notch: Notch::lone(EdgeAnchor {
            piece,
            from,
            t: 0.5,
        }),
        mate: None,
    }
    .apply(doc)
    .expect("a mark goes on the tract leaving a leg");
}

/// A wedge something else of the pattern is drawn on stays where it is.
///
/// Closing a dart back up takes its three points out of the document, and the
/// loader refuses a file that names a point it does not carry. Measured before
/// this guard: a mark put on the wedge's own leg outlived the node it was
/// pinned to, and the product saved at 82,408 bytes and never opened again.
/// Letting the dart go is the door that stays open, because it takes no node
/// with it.
#[test]
fn a_wedge_something_is_drawn_on_is_not_one_to_close_back_up() {
    let (mut doc, piece, after) = block();
    cut(piece, Some(after))
        .apply(&mut doc)
        .expect("the wedge fits");
    let (key, dart) = only_dart(&doc);
    mark(&mut doc, piece, dart.legs.0);
    let before = doc.to_canonical_json();

    let why = Command::RemoveDart { dart: key }
        .apply(&mut doc)
        .expect_err("the mark would outlive the node it is pinned to");

    assert_eq!(why, DocError::WedgeStillDrawn);
    assert_eq!(doc.to_canonical_json(), before, "and nothing was written");

    Command::UndeclareDart { dart: key }
        .apply(&mut doc)
        .expect("letting it go takes no node with it");
    Doc::from_json(&doc.to_canonical_json()).expect("and the product still opens");
}

/// The history reaches the removal from the other side, and is never refused.
///
/// The guard sits on the inverse of a cut, so the one thing it must not do is
/// block an undo. It cannot: the mark went on after the dart, so its own entry
/// is the one taken back first, and by the time the cut is reached the wedge
/// carries nothing.
#[test]
fn undoing_a_cut_under_a_mark_is_not_refused() {
    let (mut doc, piece, after) = block();
    let mut history = History::new();
    history.begin(CUT);
    history
        .edit(&mut doc, cut(piece, Some(after)))
        .expect("the wedge fits");
    history.end();
    let (_, dart) = only_dart(&doc);
    history.begin("piquete");
    history
        .edit(
            &mut doc,
            Command::AddNotch {
                identity: Identity::New,
                notch: Notch::lone(EdgeAnchor {
                    piece,
                    from: dart.legs.0,
                    t: 0.5,
                }),
                mate: None,
            },
        )
        .expect("the mark goes on");
    history.end();

    history.undo(&mut doc).expect("the mark comes off first");
    history.undo(&mut doc).expect("and then the wedge");
    assert_eq!(doc.darts.len(), 0);
    assert_eq!(doc.notches.len(), 0);
}

/// A cut wedge let go of stays drawn, and its three nodes come off one by one.
///
/// The record cannot say whether a wedge was cut by the tool or drawn by hand,
/// so the door that keeps a drafted wedge is open on a cut one too — and what
/// it leaves behind is exactly what the word promises: three nodes in the
/// contour with nothing calling them a dart. They are not orphans. The file
/// opens, and the node tool takes each of them off, which is the route a wedge
/// cut in the wrong place already had.
#[test]
fn a_cut_wedge_let_go_of_stays_drawn_and_walks_off_node_by_node() {
    let (mut doc, piece, after) = block();
    let was = contour(&doc, piece);
    cut(piece, Some(after))
        .apply(&mut doc)
        .expect("the wedge fits");
    let (key, dart) = only_dart(&doc);
    let grown = contour(&doc, piece);
    assert_eq!(grown.len(), was.len() + 3);

    Command::UndeclareDart { dart: key }
        .apply(&mut doc)
        .expect("the record and the thread come off");
    assert_eq!(doc.darts.len(), 0, "nothing calls it a dart");
    assert_eq!(contour(&doc, piece), grown, "and the wedge is still drawn");
    Doc::from_json(&doc.to_canonical_json()).expect("the product opens");

    for node in [dart.legs.0, dart.apex, dart.legs.1] {
        Command::RemoveNode { piece, node }
            .apply(&mut doc)
            .expect("no dart is written over it any more");
    }
    assert_eq!(contour(&doc, piece), was, "the contour the cut found");
    Doc::from_json(&doc.to_canonical_json()).expect("and that product opens too");
}
