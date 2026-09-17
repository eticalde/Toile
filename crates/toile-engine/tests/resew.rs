#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use std::time::{Duration, Instant};

use toile_engine::body::Collider;
use toile_engine::draft::{
    Command, Draft, Identity, PieceKey, Point, PointKey, SegmentEdit, block,
};
use toile_engine::session::Session;

/// How long a test waits on the sim thread before calling it stuck.
const PATIENCE: Duration = Duration::from_secs(120);

/// Substeps of drape before the rebuild, and as many again after it.
///
/// The first stretch is long enough for the sewing to have closed the seams,
/// which is what makes the reading before the rebuild worth comparing against.
/// The second carries the drape past the substep the sewing turns firm at, so a
/// seam the solver is no longer holding has had every chance to be pulled shut
/// and has not been.
const DRAPE: u64 = 300;

/// How far apart a sewn pair may be and still count as sewn, in metres.
///
/// Two millimetres, the most one pass of the sewing may pull a pair together:
/// a pair the solver is holding never stands further apart than the correction
/// it would have applied. The drape closes these to zero before the rebuild, so
/// this is a bound on what the rebuild did and not on the sewing.
const TOGETHER: f32 = 0.002;

/// A node for the middle of the first straight tract of a piece, a centimetre
/// off the line it splits.
///
/// Off the line and not on it, and that is the whole of what makes this test
/// able to fail. A collinear node changes what the contour is made of without
/// changing the line it draws, and the mesher samples a contour by arc length:
/// the rebuilt piece comes back with the very same vertices in the very same
/// order, nothing after it moves, and seams paired against the mesh that was
/// replaced are still, by accident, the right ones.
fn bulge(draft: &Draft, piece: PieceKey) -> (PointKey, Point) {
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
        Point::at(
            f64::midpoint(from[0], to[0]),
            f64::midpoint(from[1], to[1]) - 1.0,
        ),
    )
}

fn gap(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt()
}

/// Widest and mean separation of the sewn pairs, in the frame on the stand.
///
/// The pairs are read from the session rather than remembered from before the
/// rebuild: they are the document's seams paired onto the meshes standing there
/// now, which is exactly what the solver was supposed to have been handed.
fn seam_gaps(session: &Session) -> (f32, f32) {
    let snap = session.snapshot();
    let at = snap.positions.as_chunks::<3>().0;
    assert_eq!(
        at.len(),
        session.n_vertices(),
        "the frame is of these meshes"
    );
    let pairs = session.sewn_pairs();
    let (mut worst, mut sum) = (0.0f32, 0.0f32);
    for &(a, b) in &pairs {
        let d = gap(at[a as usize], at[b as usize]);
        worst = worst.max(d);
        sum += d;
    }
    (worst, sum / pairs.len() as f32)
}

/// Waits for a published frame of the meshes now on the stand, `past` substeps
/// into the drape.
///
/// Both halves are needed. The substep count alone would accept a frame from
/// before the swap landed, and the generation alone would accept the very first
/// frame after it, which no seam has yet had a substep to open.
fn frame_past(session: &Session, past: u64) {
    let deadline = Instant::now() + PATIENCE;
    loop {
        let snap = session.snapshot();
        if snap.generation >= session.mesh_generation()
            && snap.substeps >= past
            && snap.positions.len() == session.n_vertices() * 3
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "the sim thread never reached {past} substeps"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// A piece rebuilt in the middle of a drape comes back sewn to the rest of the
/// product.
///
/// A rebuild hands its piece a whole new set of vertices and moves the base of
/// every piece standing after it, so the seams have to be paired again against
/// the meshes the swap leaves behind. Paired against the meshes it replaces
/// they stay inside the state — the rebuilt piece grew — and the sim thread's
/// range check, which is a bounds check, takes them. Nothing refuses, nothing
/// panics, and the garment quietly comes apart.
///
/// Measured through the seams the document draws, in the frame the sim
/// publishes: the pairs the solver is meant to be holding, where the person
/// would see them.
#[test]
fn a_rebuilt_piece_is_still_sewn_to_the_rest_of_the_product() {
    let mut session =
        Session::from_doc(block::trousers(), Collider::demo()).expect("the block drapes");
    let front = session.pieces()[0];
    let pairs = session.sewn_pairs().len();
    assert!(pairs > 0, "the block's two pieces are sewn to each other");

    frame_past(&session, DRAPE);
    let (worst_before, mean_before) = seam_gaps(&session);
    println!("before the rebuild: worst {worst_before:.6} m, mean {mean_before:.6} m");
    assert!(
        worst_before < TOGETHER,
        "the sewing closed the seams before the rebuild: {worst_before} m"
    );

    let held = session.n_vertices();
    let draft = session.draft().expect("the block has a document");
    let (after, value) = bulge(draft, front);
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
    // A rebuild that gave the piece the same vertex count would leave the base
    // of every piece after it where it was, and a seam paired against the mesh
    // that was replaced would still name the right vertices. There would be
    // nothing here for the assertion below to catch.
    assert_ne!(
        session.n_vertices(),
        held,
        "the rebuild moved the back's base: {held} vertices before"
    );
    // Not the count it had: the rebuilt piece is meshed at the density its new
    // perimeter asks for, so the seams pair onto a different number of boundary
    // vertices. Holding that count fixed would make this test red for the
    // rebuild having happened at all. What has to survive is that they pair.
    assert!(
        !session.sewn_pairs().is_empty(),
        "the document's seams still pair onto the meshes on the stand"
    );

    frame_past(&session, DRAPE * 2);
    let (worst, mean) = seam_gaps(&session);
    println!("after the rebuild: worst {worst:.6} m, mean {mean:.6} m");
    assert_eq!(
        session.snapshot().refused,
        None,
        "the sim thread took the swap"
    );
    assert!(
        worst < TOGETHER,
        "the rebuilt piece was handed to the solver with the seams of the mesh \
         it replaced: a sewn pair stands {worst} m apart, {mean} m on average"
    );
}
