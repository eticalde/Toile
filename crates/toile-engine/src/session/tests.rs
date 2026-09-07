use super::*;
use crate::draft::{Axis, Binding, Command, Identity, Piece, Point, SegmentEdit, Winding, block};

/// A blank table adopts the first drawn piece: an empty piece lands, its
/// vertices follow one command at a time, and the drape starts once the
/// contour can be meshed — the whole point of the "Nuevo producto" button.
#[test]
fn a_blank_document_adopts_the_first_drawn_piece() {
    let mut session = Session::blank();
    assert!(session.piece().is_none(), "nothing drapes on a blank table");

    let piece = PieceKey::new(
        session
            .draft()
            .expect("blank has a document")
            .doc()
            .pieces
            .issued(),
        0,
    );
    session
        .edit(Command::AddPiece {
            identity: Identity::New,
            piece: Piece::polygon("Pieza 1", std::iter::empty(), Winding::Ccw),
        })
        .expect("an empty piece lands");
    assert!(session.piece().is_none(), "an empty piece cannot drape yet");

    // Head-inserted, so this call order leaves the contour
    // counter-clockwise.
    for [x, y] in [[0.15, 0.30], [0.30, 0.0], [0.0, 0.0]] {
        session
            .edit(Command::InsertNode {
                piece,
                after: None,
                identity: Identity::New,
                value: Point::at(x, y),
                segment: SegmentEdit::Line,
                samples: 1,
            })
            .expect("a vertex lands");
    }

    assert_eq!(session.piece(), Some(piece), "the finished triangle drapes");
    assert!(session.n_vertices() > 0, "and it meshed");
}

/// A document whose first piece is still too partial to mesh — the shape an
/// autosave leaves when the drawing is paused after a point or two — reopens as
/// a blank table rather than being refused, so no work is lost to a save taken
/// mid-gesture.
#[test]
fn a_partial_first_piece_reopens_as_a_blank_table() {
    let mut drawing = Session::blank();
    let piece = PieceKey::new(
        drawing
            .draft()
            .expect("blank has a document")
            .doc()
            .pieces
            .issued(),
        0,
    );
    drawing
        .edit(Command::AddPiece {
            identity: Identity::New,
            piece: Piece::polygon("Pieza 1", std::iter::empty(), Winding::Ccw),
        })
        .expect("an empty piece lands");
    for [x, y] in [[0.0, 0.0], [0.30, 0.0]] {
        drawing
            .edit(Command::InsertNode {
                piece,
                after: None,
                identity: Identity::New,
                value: Point::at(x, y),
                segment: SegmentEdit::Line,
                samples: 1,
            })
            .expect("a vertex lands");
    }
    assert!(drawing.piece().is_none(), "two points cannot mesh");

    // Through the very round-trip an autosave and a reopen take.
    let json = drawing
        .draft()
        .expect("the drawing has a document")
        .doc()
        .to_canonical_json();
    let doc = Doc::from_json(&json).expect("what an autosave writes it reads back");

    let reopened = Session::from_doc(doc).expect("a partial product still opens");
    assert!(reopened.draft().is_some(), "it carries the document");
    assert!(
        reopened.piece().is_none(),
        "with its partial piece not draping yet"
    );
}

/// The mesh is built at a topology count, and a shape edit that arrives
/// against another one has to say so rather than warm-start across it.
#[test]
fn a_stale_generation_is_an_error_not_a_warm_start() {
    let mut session = Session::from_doc(block::trouser_front()).expect("the block drapes");
    let piece = session.piece().expect("the session has a document");
    session
        .slot
        .as_mut()
        .expect("the block drapes a piece")
        .set_topology(7);
    let node = session
        .draft()
        .expect("the session has a document")
        .points_cm(piece)[1]
        .0;
    let moved = session.edit(Command::SetBinding {
        point: node,
        axis: Axis::X,
        to: Binding::literal(23.0),
    });
    assert_eq!(
        moved,
        Err(SessionError::TopologyMismatch {
            piece,
            expected: 7,
            got: 0
        })
    );
}
