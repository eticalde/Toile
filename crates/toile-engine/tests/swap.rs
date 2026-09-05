#![allow(missing_docs, reason = "a test crate publishes no API surface")]
#![allow(
    clippy::float_cmp,
    reason = "a move to a round number of centimetres lands on it exactly"
)]

use std::time::{Duration, Instant};

use toile_engine::couture::{self, COMPLIANCE, MeshSwap, ShapePipeline};
use toile_engine::draft::{
    Binding, Command, Draft, Identity, PieceKey, Point, PointKey, SegmentEdit, block,
};
use toile_engine::session::Session;
use toile_sim::xpbd::{self, SdfGrid, Seams, State};

/// Simulated seconds per substep, as the engine runs it.
const DT: f32 = 1.0 / 600.0;

/// Substeps of drape before the topology changes: long enough for the panel
/// to reach the sphere and still be moving when the mesh is pulled from under
/// it, which is the moment the swap has to survive.
const DRAPE: usize = 240;

/// Substeps of settling allowed after the swap, a tenth of a second.
const SETTLE: usize = 60;

/// How much motion one swap may add, as a multiple of the motion the drape
/// already had. The topology benchmark measures 2.6 on the demo bodice; this
/// is the ceiling that separates a re-projection from a detonation.
const SPIKE: f32 = 4.0;

/// How long a test waits on another thread before calling it stuck.
const PATIENCE: Duration = Duration::from_secs(20);

/// A panel, and the same panel after a node was put into its hem and pulled
/// a centimetre out: the topology edit the phase is named for, meshed at the
/// density the engine itself chooses.
fn panels() -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
    let plain = vec![[0.0, 0.0], [0.20, 0.0], [0.20, 0.30], [0.0, 0.30]];
    let mut cut = plain.clone();
    cut.insert(1, [0.10, -0.01]);
    (plain, cut)
}

/// Meshes a contour the way the engine does, density and all.
fn mesh(contour: &[[f64; 2]]) -> ShapePipeline {
    let (samples, max_area) = couture::for_contour(contour);
    ShapePipeline::build(contour, samples, max_area).expect("the panel is finite")
}

/// The avatar, small enough that a test does not bake a 67 MB grid.
fn sphere() -> SdfGrid {
    SdfGrid::sphere(48, 0.5 / 47.0, [-0.25, -0.25, -0.25], [0.0, 0.0, 0.0], 0.08)
}

/// Mean kinetic energy per vertex, the number the sim thread sleeps on.
fn energy(state: &State) -> f32 {
    xpbd::kinetic_energy(state) / state.len() as f32
}

/// Mean height, in metres: where the cloth is, in one number.
fn height(state: &State) -> f32 {
    state.py.iter().sum::<f32>() / state.len() as f32
}

/// Lowest and highest vertex, in metres.
fn span(state: &State) -> (f32, f32) {
    state
        .py
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), &y| (lo.min(y), hi.max(y)))
}

/// A mid-air drape carried onto a re-meshed panel keeps its position and its
/// motion: the transfer moves cloth, it does not stir it.
#[test]
fn the_drape_survives_a_mesh_swap() {
    let (plain, cut) = panels();
    let old = mesh(&plain);
    let new = mesh(&cut);
    assert_ne!(old.pos2d.len(), new.pos2d.len(), "the mesh really changed");
    let cons = old.constraints(COMPLIANCE);
    let sdf = sphere();
    let no_seams = Seams::default();

    let mut state = couture::drop_state(&old, couture::DROP_HEIGHT);
    for _ in 0..DRAPE {
        xpbd::substep(&mut state, &cons, &no_seams, &sdf, DT);
    }
    let (was_moving, was_at) = (energy(&state), height(&state));
    let (was_lowest, was_highest) = span(&state);
    assert!(
        was_moving > 1.0e-4,
        "the panel is still draping: {was_moving}"
    );

    let swap = MeshSwap::new(&old.pos2d, &old.tris, &new, COMPLIANCE);
    let mut carried = couture::onto(&swap, &state);
    assert_eq!(carried.len(), new.pos2d.len(), "the new mesh is in place");
    // The new mesh samples the old cloth at its own vertices, so the mean
    // moves a little; what it must not do is invent a fold. Every carried
    // position is a convex mixture of three old ones, and stays inside them.
    let (lowest, highest) = span(&carried);
    assert!(
        lowest >= was_lowest - 1.0e-6 && highest <= was_highest + 1.0e-6,
        "the swap invented no new extreme: {lowest}..{highest} against {was_lowest}..{was_highest}"
    );
    assert!(
        (height(&carried) - was_at).abs() < 5.0e-3,
        "the cloth stayed where it was: {} against {was_at}",
        height(&carried)
    );
    assert!(
        energy(&carried) < was_moving * 1.5,
        "the swap added no motion: {} against {was_moving}",
        energy(&carried)
    );

    // The first substep on the new mesh pays for the interpolation: the
    // carried surface is the old one's chords, so the stretch constraints
    // pull it back onto its rest lengths. It is a step, not a bang, and it is
    // paid once — a tenth of a second later the panel is quieter than it was
    // before the swap, which is the drape carrying on rather than restarting.
    xpbd::substep(&mut carried, &swap.cons, &no_seams, &sdf, DT);
    let spike = energy(&carried);
    assert!(
        spike < was_moving * SPIKE,
        "the swap cost one bounded step: {spike} against {was_moving}"
    );
    for _ in 0..SETTLE {
        xpbd::substep(&mut carried, &swap.cons, &no_seams, &sdf, DT);
    }
    assert!(
        energy(&carried) < was_moving,
        "and the panel went on settling: {} against {was_moving}",
        energy(&carried)
    );
}

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
    let mut session = Session::from_doc(block::trouser_front()).expect("the block drapes");
    let before = substeps_past(&session, 0);
    assert!(before > 0, "the sim thread is running");
    let nodes = session.contour().len();

    let command = insert(&mut session);
    session.edit(command).expect("a node goes into the contour");
    assert!(session.remeshing(), "the rebuild left the interface thread");
    assert_eq!(
        session.contour().len(),
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
    assert_eq!(session.contour().len(), nodes + 1);
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
    let mut session = Session::from_doc(block::trouser_front()).expect("the block drapes");
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
    let mut session = Session::from_doc(block::trouser_front()).expect("the block drapes");
    let vertices = session.n_vertices();
    let command = insert(&mut session);
    session.edit(command).expect("a node goes into the contour");
    session.wait_for_remesh().expect("the rebuild lands");

    let nodes = session.contour().len();
    session.undo().expect("the insertion comes back out");
    session.wait_for_remesh().expect("the rebuild lands");
    assert_eq!(session.contour().len(), nodes - 1);
    assert_eq!(session.n_vertices(), vertices, "the same mesh, rebuilt");
    assert!(!session.remeshing());
}

/// The generation the mesh was installed at is what the viewport gates
/// frames on: only a landed swap moves it, so a snapshot from before the
/// swap can be told from one of the mesh now on the table.
#[test]
fn the_mesh_generation_moves_with_the_swap_alone() {
    let mut session = Session::from_doc(block::trouser_front()).expect("the block drapes");
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
