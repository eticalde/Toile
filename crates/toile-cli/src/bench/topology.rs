use std::time::{Duration, Instant};

use toile_engine::couture::{ShapePipeline, transfer_state};
use toile_engine::demo;
use toile_engine::draft::{Command, Draft, Identity, PieceKey, Point, SegmentEdit, block};
use toile_engine::session::Session;
use toile_mesh::transfer;
use toile_sim::xpbd::{self, Seams};

use super::scene::{DT, same_bits, seconds, settle};

/// Substeps of drape before the topology change.
const DRAPE_SUBSTEPS: usize = 600;

/// The budget one topology edit has, end to end.
const BUDGET_MS: f64 = 500.0;

struct Swap {
    rebuild_ms: f64,
    energy_before: f64,
    energy_after: f64,
    reconverge_s: f64,
    hash: u64,
}

/// Adds a contour point and moves the shoulder, then carries the live drape
/// onto the new mesh.
///
/// In the engine this rebuild runs in the shadow of the old mesh while the
/// solver keeps integrating; here it is measured on its own.
fn swap() -> Swap {
    let no_seams = Seams::default();
    let contour_a = demo::bodice_contour();
    let pipe_a = demo::pipeline(&contour_a);
    let mut state = demo::drop_state(&pipe_a);
    let cons_a = pipe_a.constraints(1.0e-8);
    let sdf = demo::avatar_sdf();
    for _ in 0..DRAPE_SUBSTEPS {
        xpbd::substep(&mut state, &cons_a, &no_seams, &sdf, DT);
    }
    let n = state.len();
    let energy_before = f64::from(xpbd::kinetic_energy(&state) / n as f32);

    let mut contour_b = contour_a.clone();
    let mid = [
        f64::midpoint(contour_b[29][0], contour_b[30][0]) + 0.02,
        f64::midpoint(contour_b[29][1], contour_b[30][1]),
    ];
    contour_b.insert(30, mid);
    // The insert pushed every later index along by one.
    contour_b[demo::SHOULDER_POINT + 1][0] += 0.03;

    let t0 = Instant::now();
    let pipe_b = demo::pipeline(&contour_b);
    let cons_b = pipe_b.constraints(1.0e-8);
    let mut state_b = transfer_state(&pipe_a, &state, &pipe_b);
    let rebuild_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let nb = state_b.len();

    xpbd::substep(&mut state_b, &cons_b, &no_seams, &sdf, DT);
    let energy_after = f64::from(xpbd::kinetic_energy(&state_b) / nb as f32);

    let steps = settle(&mut state_b, &cons_b, &no_seams, &sdf, 6000);
    Swap {
        rebuild_ms,
        energy_before,
        energy_after,
        reconverge_s: seconds(steps),
        hash: xpbd::position_hash(&state_b),
    }
}

/// How badly the interior interpolator folds under extreme edits.
fn foldovers() {
    let contour = demo::bodice_contour();
    let edits: [(&str, usize, [f64; 2]); 3] = [
        (
            "hombro +2 cm (suave)     ",
            demo::SHOULDER_POINT,
            [0.02, 0.0],
        ),
        (
            "hombro -10 cm (extrema)  ",
            demo::SHOULDER_POINT,
            [-0.10, 0.0],
        ),
        ("sisa +12 cm (a través)   ", 50, [0.12, 0.0]),
    ];
    println!("\n── foldovers del interpolador (MVC) en ediciones extremas ──");
    for (name, idx, d) in edits {
        let mut pipe = demo::pipeline(&contour);
        let reference = pipe.pos2d.clone();
        let tris = pipe.tris.clone();
        let mut edited = contour.clone();
        edited[idx][0] += d[0];
        edited[idx][1] += d[1];
        pipe.derive(&edited)
            .expect("the bench moves a point, never the node count");
        let flips = transfer::count_flipped(&reference, &pipe.pos2d, &tris);
        println!(
            "{name}  {flips} triángulos invertidos de {}  {}",
            tris.len() / 3,
            if flips == 0 { "✅" } else { "⚠️" }
        );
    }
}

/// What one topology edit costs the person, stage by stage.
struct Front {
    resolve_ms: f64,
    remesh_ms: f64,
    to_solver_ms: f64,
    verts: (usize, usize),
    nodes: (usize, usize),
}

/// A node at the middle of the first straight tract, pulled a centimetre out:
/// the point tool of this phase, on the block the editor opens with.
fn inserted(session: &Session) -> Command {
    let piece = session.piece().expect("the session has a document");
    let draft = session.draft().expect("the session has a document");
    let nodes = draft.points_cm(piece);
    let seat = seat(draft, piece);
    let (after, from) = nodes[seat];
    let to = nodes[(seat + 1) % nodes.len()].1;
    Command::InsertNode {
        piece,
        after: Some(after),
        identity: Identity::New,
        value: Point::at(
            f64::midpoint(from[0], to[0]),
            f64::midpoint(from[1], to[1]) - 1.0,
        ),
        segment: SegmentEdit::Line,
        samples: 1,
    }
}

/// Where in the contour the first straight tract starts.
fn seat(draft: &Draft, piece: PieceKey) -> usize {
    draft
        .doc()
        .pieces
        .get(piece)
        .expect("the piece is live")
        .contour
        .iter()
        .position(|node| !node.segment.bends())
        .expect("the block draws at least one straight tract")
}

/// One topology edit on the trouser front, measured end to end.
///
/// This is the whole road the person walks: the document resolves, the mesher
/// rebuilds off the interface thread, and the swap reaches the solver, which
/// went on integrating the old mesh the entire time.
fn front() -> Front {
    let mut session = Session::from_doc(block::trouser_front()).expect("the block drapes");
    let verts_before = session.n_vertices();
    let nodes_before = session.contour().len();
    // A swap that carries nothing is not the case under test: the panel has to
    // be draping when the mesh is pulled from under it.
    while session.snapshot().substeps < DRAPE_SUBSTEPS as u64 {
        std::thread::sleep(Duration::from_millis(1));
    }

    let command = inserted(&session);
    let t0 = Instant::now();
    session.edit(command).expect("a node goes into the contour");
    let resolve_ms = t0.elapsed().as_secs_f64() * 1000.0;
    session
        .wait_for_remesh()
        .expect("the mesher rebuilds the piece");
    // The solver publishes the new vertex count once it has taken the swap.
    while session.snapshot().positions.len() != session.n_vertices() * 3 {
        std::thread::sleep(Duration::from_millis(1));
    }
    Front {
        resolve_ms,
        remesh_ms: session.last_remesh_ms,
        to_solver_ms: t0.elapsed().as_secs_f64() * 1000.0,
        verts: (verts_before, session.n_vertices()),
        nodes: (nodes_before, session.contour().len()),
    }
}

fn mesh_hash(p: &ShapePipeline) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |x: u64| h = (h ^ x).wrapping_mul(0x0100_0000_01b3);
    for v in &p.pos2d {
        eat(v[0].to_bits());
        eat(v[1].to_bits());
    }
    for &t in &p.tris {
        eat(u64::from(t));
    }
    h
}

pub fn run() {
    println!("\n── vía B: topología con transferencia baricéntrica ──");
    let a = swap();
    let b = swap();
    println!(
        "rebuild + transferencia {:7.1} ms  (presupuesto: <500 ms)",
        a.rebuild_ms
    );
    println!(
        "energía/vért            {:9.2e} antes del swap · {:9.2e} tras el primer substep",
        a.energy_before, a.energy_after
    );
    println!(
        "re-convergencia         {:7.2} s de sim tras el swap",
        a.reconverge_s
    );
    println!("determinismo            {}", same_bits(a.hash, b.hash));

    let front = front();
    println!("\n── delantero del pantalón · insertar un nodo a mitad de drapeado ──");
    println!(
        "contorno                {} → {} nodos · malla {} → {} vértices",
        front.nodes.0, front.nodes.1, front.verts.0, front.verts.1
    );
    println!(
        "resolver el documento   {:7.1} ms  (el hilo de UI no espera más que esto)",
        front.resolve_ms
    );
    println!("remallado en el worker  {:7.1} ms", front.remesh_ms);
    println!(
        "hasta el solver         {:7.1} ms  (presupuesto: <{BUDGET_MS:.0} ms) · {}",
        front.to_solver_ms,
        verdict(front.to_solver_ms)
    );

    foldovers();

    // Undo rebuilds from the original contour: the sampling map lives in the
    // document revision, so the mesh has to come back bit-exact.
    let contour = demo::bodice_contour();
    let (m1, m2) = (
        mesh_hash(&demo::pipeline(&contour)),
        mesh_hash(&demo::pipeline(&contour)),
    );
    println!(
        "\nundo (rebuild del contorno original): malla {}",
        if m1 == m2 {
            "bit-idéntica ✅"
        } else {
            "DIFIERE ❌"
        }
    );
}

/// Whether the edit stayed inside its budget.
fn verdict(ms: f64) -> &'static str {
    if ms < BUDGET_MS { "dentro" } else { "FUERA" }
}
