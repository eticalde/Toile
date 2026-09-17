use super::*;
use crate::draft::{
    Axis, Binding, Command, Doc, Identity, Piece, Point, SegmentEdit, Winding, block,
};

/// A blank table adopts the first drawn piece: an empty piece lands, its
/// vertices follow one command at a time, and the drape starts once the
/// contour can be meshed — the whole point of the "Nuevo producto" button.
#[test]
fn a_blank_document_adopts_the_first_drawn_piece() {
    let mut session = Session::blank(Collider::demo());
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
    let mut drawing = Session::blank(Collider::demo());
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

    let reopened = Session::from_doc(doc, Collider::demo()).expect("a partial product still opens");
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
    let mut session =
        Session::from_doc(block::trouser_front(), Collider::demo()).expect("the block drapes");
    let piece = session.piece().expect("the session has a document");
    session.draping[0].slot.set_topology(7);
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

/// The two pieces of the shipped block, and the node whose insertion rebuilds
/// the front without touching the back.
fn trousers() -> (Session, PieceKey, PieceKey) {
    let session = Session::from_doc(block::trousers(), Collider::demo()).expect("the block drapes");
    let doc = session.draft().expect("the block has a document").doc();
    let front = doc
        .piece_named(block::FRONT)
        .expect("the block draws a front");
    let back = doc
        .piece_named(block::BACK)
        .expect("the block draws a back");
    (session, front, back)
}

/// The session drapes the whole product, not its first piece: both pieces are
/// meshed into one state, the back's vertices begin past the front's, and
/// every triangle indexes that one state.
#[test]
fn a_product_of_two_pieces_drapes_both_of_them() {
    let (session, front, back) = trousers();
    assert_eq!(session.pieces(), [front, back]);
    assert_eq!(session.offset(front), Some(0), "the first piece opens it");

    let base = session.offset(back).expect("the back drapes too") as usize;
    assert!(base > 0 && base < session.n_vertices());
    assert_eq!(session.contour_m(front).len(), 47);
    assert_eq!(session.contours_m().count(), 2);

    let tris = session.triangles();
    assert!(tris.iter().all(|&v| (v as usize) < session.n_vertices()));
    assert!(
        tris.iter().any(|&v| v as usize >= base),
        "the back's triangles are rebased into the combined state, not dropped"
    );
}

/// A rebuild of one piece leaves the others on the stand as they were: the
/// back keeps the mesh and the contour it had, and its block of the combined
/// state is still its own — which is the whole reason the topology count and
/// the vertex bases are kept per piece.
#[test]
fn a_rebuild_of_one_piece_leaves_the_others_alone() {
    let (mut session, front, back) = trousers();
    let nodes = session
        .draft()
        .expect("the block has a document")
        .points_cm(front);
    let (after, from) = nodes[0];
    let to = nodes[1].1;
    let value = Point::at(
        f64::midpoint(from[0], to[0]),
        f64::midpoint(from[1], to[1]) - 1.0,
    );
    let (front_nodes, back_nodes) = (
        session.contour_m(front).len(),
        session.contour_m(back).len(),
    );
    let back_verts = session.n_vertices() - session.offset(back).expect("the back drapes") as usize;

    session
        .edit(Command::InsertNode {
            piece: front,
            after: Some(after),
            identity: Identity::New,
            value,
            segment: SegmentEdit::Line,
            samples: 1,
        })
        .expect("a node goes into the front");
    session.wait_for_remesh().expect("the rebuild lands");

    assert_eq!(session.pieces(), [front, back], "nobody left the stand");
    assert_eq!(session.contour_m(front).len(), front_nodes + 1);
    assert_eq!(
        session.contour_m(back).len(),
        back_nodes,
        "the back was never re-meshed"
    );
    let base = session.offset(back).expect("the back still drapes") as usize;
    assert_eq!(
        session.n_vertices() - base,
        back_verts,
        "and it still holds exactly its own block of the state"
    );
    assert!(session.mesh_generation() > 0, "the swap landed");
    let tris = session.triangles();
    assert!(tris.iter().all(|&v| (v as usize) < session.n_vertices()));
}

/// The seams and the body place the product: the two pieces are rolled onto
/// opposite sides of one ring, and that ring is the garment's own size.
///
/// Front before the body and back behind it are not declared anywhere. The
/// document says only which stretch of one meets which stretch of the other,
/// and walking that ring is what puts them across the axis from each other.
///
/// The demo ball carries no measurements, so no ring on it can be matched: the
/// product is let go about the whole of it, at its own cloth's size, opened
/// only as far as the ball makes necessary — well inside the ball's own box.
#[test]
fn the_seams_put_one_piece_before_the_body_and_one_behind_it() {
    let (session, front, back) = trousers();
    let split = session.offset(back).expect("the back drapes") as usize;
    let released = session.released();
    let at = released.as_chunks::<3>().0;
    assert_eq!(at.len(), session.n_vertices());
    assert_eq!(session.offset(front), Some(0));

    let ring = session.layout().expect("the seams place the product");
    let mean_z = |run: &[[f32; 3]]| run.iter().map(|p| p[2]).sum::<f32>() / run.len() as f32;
    assert!(mean_z(&at[..split]) > 0.0, "the front is before the body");
    assert!(mean_z(&at[split..]) < 0.0, "the back is behind it");
    for p in at {
        let r = f64::from(p[0].hypot(p[2]));
        assert!((r - ring.radius).abs() < 1.0e-5, "a vertex sits at {r}");
    }
    assert!(
        ring.radius < f64::from(0.15f32.hypot(0.15)),
        "the ring is the cloth's own size and not the ball's: {}",
        ring.radius
    );
}

/// And the seams reach the solver: every document seam pairs, into the one
/// index space the combined state uses.
#[test]
fn the_block_arrives_at_the_solver_sewn() {
    let (session, front, back) = trousers();
    let split = session.offset(back).expect("the back drapes");
    let pairs = session.sewn_pairs();
    assert!(session.seam_faults().is_empty(), "the block's seams pair");
    assert!(!pairs.is_empty(), "and they reach the solver");
    for &(a, b) in &pairs {
        assert!(a < split, "side a stays in the front's block");
        assert!(
            (split..session.n_vertices() as u32).contains(&b),
            "side b stays in the back's"
        );
    }
    assert_eq!(session.contour_m(front).len(), 47);
}

/// A lone piece has no partner to be placed against, so nothing places it: it
/// is let go flat, where it is let go today. This is the reduction the drape
/// goldens stand on, asserted through the session rather than under it.
#[test]
fn a_product_of_one_piece_is_still_let_go_flat() {
    let session =
        Session::from_doc(block::trouser_front(), Collider::demo()).expect("the block drapes");
    assert!(session.sewn_pairs().is_empty(), "nothing is sewn to it");
    let released = session.released();
    let at = released.as_chunks::<3>().0;
    let height = session.collider().release_height();
    assert!(at.iter().all(|p| (p[1] - height).abs() < 1.0e-7));
}

/// A piece the document lets go of leaves the stand, and the rest of the
/// product goes on draping without it.
#[test]
fn a_removed_piece_leaves_the_stand_and_the_others_stay() {
    let (mut session, front, back) = trousers();
    session
        .edit(Command::RemovePiece { piece: back })
        .expect("the back comes off the table");
    assert_eq!(session.pieces(), [front]);
    assert_eq!(session.offset(back), None);
    assert_eq!(session.offset(front), Some(0));
    assert_eq!(session.triangles().len() % 3, 0);
    assert!(session.simulating(), "the front is still on the stand");
}
