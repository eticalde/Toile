use std::time::{Duration, Instant};

use toile_engine::body::Collider;
use toile_engine::draft::{
    Binding, Command, Draft, Identity, PieceKey, Point, PointKey, SegmentEdit, block,
};
use toile_engine::session::Session;

use crate::PATIENCE;

/// A node at the middle of the first straight tract of a piece.
///
/// Collinear on purpose: the edit changes what the contour is made of without
/// changing the line it draws, so the rebuild is a pure topology change.
fn midpoint(draft: &Draft, piece: PieceKey) -> (PointKey, Point) {
    let held = draft.doc().pieces.get(piece).expect("the piece is live");
    let nodes = draft.points_cm(piece);
    let seat = held
        .contour
        .iter()
        .position(|node| !node.segment.bends())
        .expect("the block draws at least one straight tract");
    let (key, from) = nodes[seat];
    let to = nodes[(seat + 1) % nodes.len()].1;
    (
        key,
        Point::at(f64::midpoint(from[0], to[0]), f64::midpoint(from[1], to[1])),
    )
}

/// Puts that node into the contour: one topology edit, on the real path.
fn insert(session: &mut Session) -> Command {
    let piece = session.piece().expect("the session has a document");
    let draft = session.draft().expect("the session has a document");
    let (after, value) = midpoint(draft, piece);
    Command::InsertNode {
        piece,
        after: Some(after),
        identity: Identity::New,
        value,
        segment: SegmentEdit::Line,
        samples: 1,
    }
}

/// Waits until the sim thread has run past `mark` substeps.
fn substeps_past(session: &Session, mark: u64) -> u64 {
    let deadline = Instant::now() + PATIENCE;
    loop {
        let now = session.snapshot().substeps;
        if now > mark || Instant::now() > deadline {
            return now;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// The whole point of the shadow rebuild: the solver never waits for it.
#[test]
fn the_solver_keeps_integrating_while_the_worker_meshes() {
    let mut session =
        Session::from_doc(block::trouser_front(), Collider::demo()).expect("the block drapes");
    let before = substeps_past(&session, 0);
    assert!(before > 0, "the sim thread is running");
    let piece = session.piece().expect("the block drapes a piece");
    let nodes = session.contour_m(piece).len();

    let command = insert(&mut session);
    session.edit(command).expect("a node goes into the contour");
    assert!(session.remeshing(), "the rebuild left the interface thread");
    assert_eq!(
        session.contour_m(piece).len(),
        nodes,
        "the solver still holds the contour it was meshed from"
    );

    // Nothing has been collected, so the rebuild is still out — and the
    // solver has gone on integrating the mesh it already had.
    let during = substeps_past(&session, before);
    assert!(during > before, "the solver kept integrating: {during}");
    assert!(session.remeshing(), "and the rebuild is still in flight");

    assert!(session.wait_for_remesh().expect("the rebuild lands"));
    assert!(!session.remeshing());
    assert_eq!(session.contour_m(piece).len(), nodes + 1);
    assert!(session.last_remesh_ms > 0.0);

    // The proof that the swap actually landed: a shape edit derives against
    // the new mesh, which the old one would have refused by node count.
    let moved = shift(&session);
    session.edit(moved).expect("the drag lands on the new mesh");
    assert!(session.last_derive_ms > 0.0);
}

/// A drag that arrives while the mesher is working is not lost: it waits for
/// the mesh it belongs to and reaches the solver with it.
#[test]
fn a_drag_during_the_rebuild_reaches_the_new_mesh() {
    let mut session =
        Session::from_doc(block::trouser_front(), Collider::demo()).expect("the block drapes");
    let command = insert(&mut session);
    session.edit(command).expect("a node goes into the contour");

    let moved = shift(&session);
    session
        .edit(moved)
        .expect("a drag during a rebuild is taken, not refused");
    assert_eq!(session.last_derive_ms, 0.0, "and not derived yet");

    session.wait_for_remesh().expect("the rebuild lands");
    assert!(session.last_derive_ms > 0.0, "the drag went out with it");
    let piece = session.piece().expect("the session has a document");
    let draft = session.draft().expect("the session has a document");
    assert_eq!(draft.points_cm(piece)[0].1[0], SHIFTED);
}

/// Where `shift` puts the first node of the piece, in centimetres.
const SHIFTED: f64 = 2.5;

/// Moves the first node of the piece: a shape edit, on any topology.
fn shift(session: &Session) -> Command {
    let piece = session.piece().expect("the session has a document");
    let draft = session.draft().expect("the session has a document");
    let (point, at) = draft.points_cm(piece)[0];
    Command::MovePoint {
        point,
        to: [Binding::literal(SHIFTED), Binding::literal(at[1])],
    }
}

/// Undo is a topology edit like any other: it goes to the mesher, and the
/// piece comes back with the mesh it had before the insertion.
#[test]
fn undoing_an_insertion_meshes_the_piece_back() {
    let mut session =
        Session::from_doc(block::trouser_front(), Collider::demo()).expect("the block drapes");
    let vertices = session.n_vertices();
    let command = insert(&mut session);
    session.edit(command).expect("a node goes into the contour");
    session.wait_for_remesh().expect("the rebuild lands");

    let piece = session.piece().expect("the block drapes a piece");
    let nodes = session.contour_m(piece).len();
    session.undo().expect("the insertion comes back out");
    session.wait_for_remesh().expect("the rebuild lands");
    assert_eq!(session.contour_m(piece).len(), nodes - 1);
    assert_eq!(session.n_vertices(), vertices, "the same mesh, rebuilt");
    assert!(!session.remeshing());
}

/// The generation the mesh was installed at is what the viewport gates
/// frames on: only a landed swap moves it, so a snapshot from before the
/// swap can be told from one of the mesh now on the table.
#[test]
fn the_mesh_generation_moves_with_the_swap_alone() {
    let mut session =
        Session::from_doc(block::trouser_front(), Collider::demo()).expect("the block drapes");
    assert_eq!(session.mesh_generation(), 0);
    let moved = shift(&session);
    session.edit(moved).expect("a shape edit derives");
    assert_eq!(
        session.mesh_generation(),
        0,
        "a shape edit does not move it"
    );
    let command = insert(&mut session);
    session.edit(command).expect("a node goes into the contour");
    session.wait_for_remesh().expect("the rebuild lands");
    assert!(session.mesh_generation() > 0, "the landed swap moves it");
}
