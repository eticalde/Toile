#![allow(missing_docs, reason = "a test crate publishes no API surface")]
#![allow(
    clippy::float_cmp,
    reason = "a move to a round number of centimetres lands on it exactly"
)]

use toile_engine::body::Collider;
use toile_engine::draft::{
    Binding, Command, Doc, Identity, MannequinKey, MeasureSet, Piece, PieceKey, Point, SegmentEdit,
    Winding, block,
};
use toile_engine::session::{Session, SessionError};

fn front() -> Session {
    Session::from_doc(block::trouser_front(), Collider::demo()).expect("the block drapes")
}

#[test]
fn a_document_session_meshes_the_piece_the_draft_resolved() {
    let session = front();
    let piece = session.piece().expect("the block drapes a piece");
    // The contour the mesher takes is the flattening, not the nine nodes.
    assert_eq!(session.contour_m(piece).len(), 47);
    assert!(session.n_vertices() > 47);
    assert!(session.triangles().len().is_multiple_of(3));
    assert!(session.draft().is_some());
}

#[test]
fn changing_the_body_is_a_shape_edit_the_session_takes_in_its_stride() {
    let mut session = front();
    let before = session.n_vertices();
    let other = session
        .draft()
        .expect("the session has a document")
        .doc()
        .mannequin_named("Talla 42")
        .expect("the block carries a second body");
    session
        .edit(Command::ResolveWith { mannequin: other })
        .expect("another body is a change of shape");
    assert_eq!(session.n_vertices(), before, "the mesh was not rebuilt");
    assert!(session.last_derive_ms > 0.0);
}

#[test]
fn moving_a_node_writes_the_document_and_re_derives() {
    let mut session = front();
    let piece = session.piece().expect("the session has a document");
    let node = session
        .draft()
        .expect("the session has a document")
        .points_cm(piece)[1]
        .0;
    session
        .edit(Command::MovePoint {
            point: node,
            to: [Binding::literal(30.0), Binding::literal(0.0)],
        })
        .expect("moving a node is a change of shape");
    let draft = session.draft().expect("the session has a document");
    assert_eq!(draft.resolved(node), Some([30.0, 0.0]));
    assert_eq!(session.contour_m(piece)[1], [0.30, 0.0]);
}

#[test]
fn an_edit_on_a_demo_session_is_refused_rather_than_ignored() {
    let mut session = Session::demo_bodice();
    let refused = session.edit(Command::ResolveWith {
        mannequin: MannequinKey::new(0, 0),
    });
    assert_eq!(refused, Err(SessionError::NoDocument));
    assert!(session.draft().is_none());
}

#[test]
fn a_document_that_draws_nothing_opens_as_a_blank_table() {
    let doc = Doc::new(MeasureSet::default());
    let session = Session::from_doc(doc, Collider::demo()).expect("an empty document opens blank");
    assert!(session.draft().is_some(), "it carries the document");
    assert!(session.piece().is_none(), "with nothing draping yet");
    assert_eq!(session.n_vertices(), 0);
}

/// Draws one triangle `wide` metres across onto the table, and hands back the
/// piece it became.
fn draw(session: &mut Session, name: &str, wide: f64) -> PieceKey {
    session
        .edit(Command::AddPiece {
            identity: Identity::New,
            piece: Piece::polygon(name, std::iter::empty(), Winding::Ccw),
        })
        .expect("an empty piece lands");
    let piece = *session
        .draft()
        .expect("blank has a document")
        .doc()
        .piece_keys()
        .last()
        .expect("the piece landed");
    // Head-inserted, so this call order leaves the contour counter-clockwise.
    for [x, y] in [[wide / 2.0, 0.30], [wide, 0.0], [0.0, 0.0]] {
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
    piece
}

/// Two pieces that nothing sews together are let go in the same column.
///
/// The placement reads the seams, so pieces that do not chain into one strip
/// have no ring to be rolled onto, and each falls back to the flat release:
/// centred on its own vertices, at the height the body decides. Two of them
/// therefore land in the same space.
///
/// Left that way deliberately, and pinned here so it is not rediscovered.
/// Moving them apart would buy an honest-looking release frame and nothing
/// after it: the solver's only contact is against the body's field, so two
/// panels pass through each other wherever they meet, stacked or side by side.
/// What is missing is cloth-cloth contact, not a placement — and nothing in the
/// document says how two unsewn pieces are meant to be worn, a piece's stored
/// position being layout that never enters the drape.
#[test]
fn two_pieces_that_nothing_sews_together_are_let_go_in_the_same_column() {
    let mut session = Session::blank(Collider::demo());
    let first = draw(&mut session, "Pieza 1", 0.30);
    let second = draw(&mut session, "Pieza 2", 0.20);
    assert_eq!(session.pieces(), [first, second], "both drape");
    assert!(
        session.sewn_pairs().is_empty(),
        "nothing sews them together"
    );
    assert!(session.layout().is_none(), "so nothing places them");

    let split = session.offset(second).expect("the second drapes") as usize;
    let released = session.released();
    let at = released.as_chunks::<3>().0;
    let height = session.collider().release_height();
    assert!(at.iter().all(|p| (p[1] - height).abs() < 1.0e-7));

    let span = |run: &[[f32; 3]]| {
        run.iter()
            .fold((f32::MAX, f32::MIN, 0.0f32), |(lo, hi, m), p| {
                (lo.min(p[0]), hi.max(p[0]), m + p[0] / run.len() as f32)
            })
    };
    let (lo_a, hi_a, mean_a) = span(&at[..split]);
    let (lo_b, hi_b, mean_b) = span(&at[split..]);
    assert!(
        mean_a.abs() < 1.0e-3 && mean_b.abs() < 1.0e-3,
        "both centred"
    );
    assert!(
        lo_a < hi_b && lo_b < hi_a,
        "the two panels are released across the same ground: {lo_a}..{hi_a} \
         and {lo_b}..{hi_b}"
    );
}
