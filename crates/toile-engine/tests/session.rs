#![allow(missing_docs, reason = "a test crate publishes no API surface")]
#![allow(
    clippy::float_cmp,
    reason = "a move to a round number of centimetres lands on it exactly"
)]

use toile_engine::draft::{Command, Doc, MannequinKey, MeasureSet, block};
use toile_engine::session::{Session, SessionError};

fn front() -> Session {
    Session::from_doc(block::trouser_front()).expect("the block drapes")
}

#[test]
fn a_document_session_meshes_the_piece_the_draft_resolved() {
    let session = front();
    assert_eq!(session.contour().len(), 9);
    assert!(session.n_vertices() > 9);
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
    session.move_point(1, [0.30, 0.0]);
    let draft = session.draft().expect("the session has a document");
    assert_eq!(draft.resolved(node), Some([30.0, 0.0]));
    assert_eq!(session.contour()[1], [0.30, 0.0]);
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
fn a_document_that_draws_nothing_has_nothing_to_drape() {
    let doc = Doc::new(MeasureSet::default());
    assert_eq!(Session::from_doc(doc).err(), Some(SessionError::NoPiece));
}
