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
