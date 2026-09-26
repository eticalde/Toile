use super::*;

/// The three refusals the document makes of a cut, each of them before anything
/// moves at all.
#[test]
fn a_cut_no_contour_could_take_writes_nothing() {
    let (mut doc, piece, node) = block();
    let before = doc.clone();
    let stray = PointKey::new(99, 0);
    let flat = DartWedge {
        nodes: [
            WedgeNode::line(Identity::New, Point::at(51.0, 0.0)),
            WedgeNode::line(Identity::New, Point::at(52.0, 12.0)),
            WedgeNode::line(Identity::New, Point::at(51.0, 0.0)),
        ],
        ..wedge(piece, Some(node))
    };
    let cases = [
        (
            wedge(PieceKey::new(9, 0), None),
            DocError::stale(PieceKey::new(9, 0)),
        ),
        (wedge(piece, Some(stray)), DocError::NoSuchNode),
        (flat, DocError::FlatWedge),
    ];
    for (asking, why) in cases {
        let refused = Command::AddDart {
            identity: Identity::New,
            dart: asked(),
            wedge: Box::new(asking),
        }
        .apply(&mut doc);
        assert_eq!(refused, Err(why));
        assert_eq!(doc, before, "nothing moved");
    }
}

/// A dart's own seam cannot be unpicked on its own.
///
/// The loader refuses a dart whose seam is gone, so an editor that allowed it
/// would let a person save a product and never open it again — three presses
/// from the tool that cut the dart. The way out of a dart is the dart.
#[test]
fn the_seam_that_shuts_a_dart_is_not_one_to_unpick() {
    let (mut doc, piece, after) = block();
    cut(piece, Some(after))
        .apply(&mut doc)
        .expect("the wedge fits");
    let (_, dart) = only_dart(&doc);
    let before = doc.to_canonical_json();

    let why = Command::RemoveSeam { seam: dart.seam }
        .apply(&mut doc)
        .expect_err("the dart's own thread stays where it is");

    assert_eq!(why, DocError::SeamClosesADart);
    assert_eq!(doc.to_canonical_json(), before, "and nothing was written");
}

/// Nothing may leave a dart the contour no longer describes.
///
/// A dart's record names three nodes standing together and the loader refuses
/// a document where they do not, so each of these has to be refused where it
/// is asked and not when the file is opened — the rule `remove_seam` already
/// writes down for a dart's thread, applied to the three nodes it is sewn
/// through. Measured before the guards: an insertion inside the wedge, a
/// second wedge cut into it and a piece taken out from under one all wrote
/// files that saved and never opened again.
#[test]
fn no_edit_leaves_a_dart_its_contour_does_not_describe() {
    let (mut doc, piece, after) = block();
    cut(piece, Some(after))
        .apply(&mut doc)
        .expect("the wedge fits");
    let (_, dart) = only_dart(&doc);
    let before = doc.to_canonical_json();

    let leg = dart.legs.0;
    for asked in [
        Command::InsertNode {
            piece,
            after: Some(leg),
            identity: Identity::New,
            value: Point::at(51.5, 1.0),
            segment: SegmentEdit::Line,
            samples: 1,
        },
        Command::RemoveNode {
            piece,
            node: dart.apex,
        },
        Command::RemovePiece { piece },
        cut(piece, Some(leg)),
    ] {
        let named = format!("{asked:?}");
        let why = asked
            .apply(&mut doc)
            .expect_err(&format!("refused: {named}"));
        assert_eq!(why, DocError::InsideAWedge, "{named}");
        assert_eq!(doc.to_canonical_json(), before, "nothing written: {named}");
    }
}
