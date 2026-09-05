use std::time::Instant;

use toile_engine::couture::{self, ShapePipeline};
use toile_engine::demo;
use toile_engine::draft::{Axis, Binding, Command, Draft, PieceKey, PointKey, block};
use toile_sim::xpbd::{self, Seams};

use super::scene::{DT, avg, max, same_bits, seconds, settle};

/// Frames of the drag storm, at 60 Hz.
const FRAMES: u32 = 120;

/// Amplitude of the oscillation applied to the waist node, in centimetres.
const AMPLITUDE: f64 = 3.0;

/// Substeps of initial drape before the storm starts.
const DRAPE_SUBSTEPS: usize = 600;

/// The node the storm takes hold of: the waist end of the side seam, whose x
/// the block writes as a formula, so the storm crosses the parametric path.
const GRABBED: &str = "cintura_lat";

/// The budget one shape edit has on the interface thread, in milliseconds.
const BUDGET_MS: f64 = 5.0;

/// The block resolved, the piece it draws, and the node the storm moves.
fn drafted() -> (Draft, PieceKey, PointKey) {
    let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
    let piece = draft
        .doc()
        .piece_named(block::FRONT)
        .expect("the block draws one piece");
    let node = draft
        .doc()
        .shows_label(piece, GRABBED)
        .expect("the block names the waist");
    (draft, piece, node)
}

/// Where the waist node sits on frame `f` of the storm, in centimetres.
fn storm_x(base: f64, f: u32) -> f64 {
    let t = f64::from(f) / f64::from(FRAMES);
    base + AMPLITUDE * (t * std::f64::consts::TAU * 2.0).sin()
}

struct Storm {
    build_ms: f64,
    n_boundary: usize,
    n_interior: usize,
    n_edges: usize,
    resolve_ms: Vec<f64>,
    derive_ms: Vec<f64>,
    converge_s: f64,
    hash: u64,
}

/// One drag storm over the trouser front, edited through the command path.
///
/// The edit goes in as a command and comes back out as an outline, which is
/// the same road the drafting table takes: whatever the resolution costs is
/// counted here rather than assumed away.
fn storm() -> Storm {
    let no_seams = Seams::default();
    let (mut draft, piece, node) = drafted();
    let contour = draft.outline(piece).to_vec();
    let (samples, max_area) = couture::for_contour(&contour);

    let t0 = Instant::now();
    let mut pipe = ShapePipeline::build(&contour, samples, max_area).expect("the block meshes");
    let build_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let mut state = couture::drop_state(&pipe, couture::DROP_HEIGHT);
    let mut cons = pipe.constraints(1.0e-8);
    let sdf = demo::avatar_sdf();
    for _ in 0..DRAPE_SUBSTEPS {
        xpbd::substep(&mut state, &cons, &no_seams, &sdf, DT);
    }

    let base = draft.resolved(node).expect("the waist resolves")[0];
    let mut resolve_ms = Vec::with_capacity(FRAMES as usize);
    let mut derive_ms = Vec::with_capacity(FRAMES as usize);
    for f in 0..FRAMES {
        let moved = Command::SetBinding {
            point: node,
            axis: Axis::X,
            to: Binding::literal(storm_x(base, f)),
        };
        let t0 = Instant::now();
        draft.edit(moved).expect("the storm moves a live node");
        resolve_ms.push(t0.elapsed().as_secs_f64() * 1000.0);

        let t0 = Instant::now();
        let rests = pipe
            .derive(draft.outline(piece))
            .expect("the storm moves a point, never the node count");
        cons.rest.copy_from_slice(rests);
        derive_ms.push(t0.elapsed().as_secs_f64() * 1000.0);
        for _ in 0..10 {
            xpbd::substep(&mut state, &cons, &no_seams, &sdf, DT);
        }
    }

    let steps = settle(&mut state, &cons, &no_seams, &sdf, 6000);
    Storm {
        build_ms,
        n_boundary: pipe.n_boundary(),
        n_interior: pipe.n_interior(),
        n_edges: pipe.edges.len(),
        resolve_ms,
        derive_ms,
        converge_s: seconds(steps),
        hash: xpbd::position_hash(&state),
    }
}

/// The same storm, on the document the editor actually opens.
pub fn run() {
    let a = storm();
    let b = storm();
    let edit_ms: Vec<f64> = a
        .resolve_ms
        .iter()
        .zip(&a.derive_ms)
        .map(|(resolve, derive)| resolve + derive)
        .collect();
    println!("\n── documento del pantalón · drag storm {FRAMES} frames ──");
    println!(
        "malla            {} frontera + {} interior · {} aristas",
        a.n_boundary, a.n_interior, a.n_edges
    );
    println!(
        "build (una vez)  {:7.1} ms (CDT + clasificación + matriz MVC)",
        a.build_ms
    );
    println!(
        "resolver doc     {:7.3} ms promedio · {:.3} ms máximo",
        avg(&a.resolve_ms),
        max(&a.resolve_ms)
    );
    println!(
        "derive por edit  {:7.3} ms promedio · {:.3} ms máximo",
        avg(&a.derive_ms),
        max(&a.derive_ms)
    );
    println!(
        "edición completa {:7.3} ms máximo  (presupuesto: <{BUDGET_MS} ms) · {}",
        max(&edit_ms),
        verdict(max(&edit_ms))
    );
    println!(
        "re-convergencia  {:7.2} s de sim tras soltar  (presupuesto: 2–3 s)",
        a.converge_s
    );
    println!(
        "determinismo     storm completo: {}",
        same_bits(a.hash, b.hash)
    );
}

/// Whether the worst frame of the storm stayed inside its budget.
fn verdict(worst_ms: f64) -> &'static str {
    if worst_ms < BUDGET_MS {
        "dentro"
    } else {
        "FUERA"
    }
}
