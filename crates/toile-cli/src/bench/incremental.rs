use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use toile_doc::{Binding, Command, Doc, MeasureSet, Piece, PieceKey, Point, PointKey, Winding};
use toile_engine::{demo, sync};
use toile_sim::xpbd::{self, Seams};

use super::scene::{DT, avg, max, same_bits, seconds, settle};

/// Frames of the drag storm, at 60 Hz.
const FRAMES: u32 = 120;

/// Amplitude of the oscillation applied to the shoulder point, in metres.
const AMPLITUDE: f64 = 0.03;

/// Substeps of initial drape before the storm starts.
const DRAPE_SUBSTEPS: usize = 600;

/// Centimetres per metre: the document is in centimetres and the solver is
/// in metres, and this bench crosses that line the same way the editor does.
const CM: f64 = 100.0;

/// The document the storm edits: the demo bodice, one literal point per node.
fn bodice_doc() -> (Doc, PieceKey, Vec<PointKey>) {
    let mut doc = Doc::new(MeasureSet::default());
    let points: Vec<PointKey> = demo::bodice_contour()
        .into_iter()
        .map(|[x, y]| doc.points.insert(Point::at(x * CM, y * CM)))
        .collect();
    let piece = doc
        .pieces
        .insert(Piece::polygon("Corpiño", points.clone(), Winding::Ccw));
    (doc, piece, points)
}

/// The piece resolved into the metres the pipeline consumes.
fn outline(doc: &Doc, piece: PieceKey) -> Vec<[f64; 2]> {
    let env = BTreeMap::new();
    let held = doc
        .pieces
        .get(piece)
        .expect("the bench holds its own piece");
    held.anchors()
        .map(|key| {
            let point = doc.points.get(key).expect("the contour cites live points");
            let read =
                |binding: &Binding| binding.eval(&env).expect("the bench binds literals only") / CM;
            [read(&point.x), read(&point.y)]
        })
        .collect()
}

/// Moves one node of the piece, in metres, through the command path.
fn move_node(doc: &mut Doc, point: PointKey, to: [f64; 2]) {
    Command::MovePoint {
        point,
        to: [Binding::literal(to[0] * CM), Binding::literal(to[1] * CM)],
    }
    .apply(doc)
    .expect("the storm moves a live point");
}

/// Where the shoulder point sits on frame `f` of the storm.
fn storm_point(base: [f64; 2], f: u32) -> [f64; 2] {
    let t = f64::from(f) / f64::from(FRAMES);
    let osc = AMPLITUDE * (t * std::f64::consts::TAU * 2.0).sin();
    [base[0] + osc, base[1] + osc * 0.5]
}

struct Storm {
    build_ms: f64,
    n_boundary: usize,
    n_interior: usize,
    n_edges: usize,
    derive_ms: Vec<f64>,
    converge_s: f64,
    hash: u64,
}

/// The synchronous path: derive and simulate on this thread, so the numbers
/// are the pipeline's own cost with no scheduling noise.
fn storm() -> Storm {
    let no_seams = Seams::default();
    let (mut doc, piece, points) = bodice_doc();
    let mut contour = outline(&doc, piece);

    let t0 = Instant::now();
    let mut pipe = demo::pipeline(&contour);
    let build_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let mut state = demo::drop_state(&pipe);
    let mut cons = pipe.constraints(1.0e-8);
    let sdf = demo::avatar_sdf();
    for _ in 0..DRAPE_SUBSTEPS {
        xpbd::substep(&mut state, &cons, &no_seams, &sdf, DT);
    }

    let base = contour[demo::SHOULDER_POINT];
    let mut derive_ms = Vec::with_capacity(FRAMES as usize);
    for f in 0..FRAMES {
        move_node(&mut doc, points[demo::SHOULDER_POINT], storm_point(base, f));
        contour = outline(&doc, piece);
        let t0 = Instant::now();
        let rests = pipe
            .derive(&contour)
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
        derive_ms,
        converge_s: seconds(steps),
        hash: xpbd::position_hash(&state),
    }
}

pub fn run_sync() {
    let a = storm();
    let b = storm();
    println!("\n── vía A síncrona · drag storm {FRAMES} frames ──");
    println!(
        "malla            {} frontera + {} interior · {} aristas",
        a.n_boundary, a.n_interior, a.n_edges
    );
    println!(
        "build (una vez)  {:7.1} ms (CDT + clasificación + matriz MVC)",
        a.build_ms
    );
    println!(
        "derive por edit  {:7.3} ms promedio · {:.3} ms máximo  (presupuesto: <5 ms)",
        avg(&a.derive_ms),
        max(&a.derive_ms)
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

/// The asynchronous path: the real sim thread, the real mailbox, and a storm
/// paced against the wall clock — which is what a user actually feels.
pub fn run_async() {
    let (mut doc, piece, points) = bodice_doc();
    let mut contour = outline(&doc, piece);
    let mut pipe = demo::pipeline(&contour);
    let n = pipe.pos2d.len();
    let cons = pipe.constraints(1.0e-8);
    let n_edges = cons.len();
    let handle = sync::spawn(
        demo::drop_state(&pipe),
        cons,
        demo::avatar_sdf(),
        pipe.tris.clone(),
        DT,
        10,
    );

    let t0 = Instant::now();
    wait_for_sleep(&handle);
    let initial = t0.elapsed().as_secs_f64();

    let base = contour[demo::SHOULDER_POINT];
    let frame_dur = Duration::from_micros(16_667);
    let mut latency_ms = Vec::with_capacity(FRAMES as usize);
    let mut derive_ms = Vec::with_capacity(FRAMES as usize);
    for f in 0..FRAMES {
        let frame_start = Instant::now();
        move_node(&mut doc, points[demo::SHOULDER_POINT], storm_point(base, f));
        contour = outline(&doc, piece);

        let td = Instant::now();
        let rests = pipe
            .derive(&contour)
            .expect("the storm moves a point, never the node count")
            .to_vec();
        derive_ms.push(td.elapsed().as_secs_f64() * 1000.0);

        let generation = u64::from(f) + 1;
        let sent = Instant::now();
        handle.send_rests(generation, rests);
        latency_ms.push(wait_for_generation(&handle, generation, sent));

        std::thread::sleep(frame_dur.saturating_sub(frame_start.elapsed()));
    }

    let t0 = Instant::now();
    wait_for_sleep(&handle);
    let reconv = t0.elapsed().as_secs_f64();
    let snap = handle.snapshot();
    let nan_free = snap.positions.iter().all(|x| x.is_finite());
    let asleep = snap.converged;
    handle.stop();

    println!("\n── pipeline asíncrono · storm a 60 Hz reales ──");
    println!(
        "malla            {n} vértices · {n_edges} aristas · sim en hilo propio (10 substeps/tick)"
    );
    println!("drapeado inicial {initial:7.2} s de reloj hasta dormir  (presupuesto: <10 s)");
    println!(
        "derive (hilo UI) {:7.3} ms promedio · {:.3} ms máximo",
        avg(&derive_ms),
        max(&derive_ms)
    );
    println!(
        "latencia edición {:7.3} ms promedio · {:.3} ms máximo  (presupuesto: <200 ms)",
        avg(&latency_ms),
        max(&latency_ms)
    );
    println!("re-convergencia  {reconv:7.2} s de reloj tras soltar  (presupuesto: 2–3 s)");
    println!(
        "estado final     {} · sim dormida: {}",
        if nan_free { "sin NaN" } else { "NaN!" },
        if asleep { "sí (0% CPU)" } else { "no" }
    );
}

fn wait_for_sleep(handle: &sync::SimHandle) {
    let t0 = Instant::now();
    while !handle.snapshot().converged && t0.elapsed() < Duration::from_secs(15) {
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Milliseconds until a snapshot carrying `generation` appears, capped so a
/// stalled thread cannot hang the benchmark.
fn wait_for_generation(handle: &sync::SimHandle, generation: u64, sent: Instant) -> f64 {
    loop {
        if handle.snapshot().generation >= generation || sent.elapsed() > Duration::from_millis(500)
        {
            return sent.elapsed().as_secs_f64() * 1000.0;
        }
        std::thread::sleep(Duration::from_micros(100));
    }
}
