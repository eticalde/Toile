#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use std::time::Instant;

use toile_engine::body::{BodyMesh, Collider, Phenotype, bake, body_mesh};
use toile_engine::couture::{self, COMPLIANCE, ShapePipeline};
use toile_engine::draft::{Draft, block};
use toile_sim::xpbd::{self, Floor, KineticDamper, Layers, SdfGrid, Seams, Stage, State};

/// Simulated seconds per substep, as the engine runs it.
const DT: f32 = 1.0 / 600.0;

/// Substeps the garment is given to come down the body and heap on the
/// ground. Twenty simulated seconds: the panel is well settled by then.
const STEPS: usize = 12_000;

/// Substeps between two readings of the crossing count.
const WATCH: usize = 250;

/// Substeps timed for the cost reading: one simulated second.
const TIMED: usize = 600;

/// Substeps the heap is watched for: a hundred simulated seconds.
const WATCHED: usize = 60_000;

/// Substep the heap has to be quiet from. The panel leaves the body and lands
/// a little before twelve thousand, so this is the landing and as long again.
const QUIET_FROM: usize = 24_000;

/// Substeps one line of the energy report covers.
const LINE: usize = 6_000;

/// Mean kinetic energy per vertex a landed heap has to stay under.
///
/// A bar on the energy itself, and not the sim thread's own rule — that one
/// reads how far a vertex travelled across a tick, so a heap held apart could
/// meet it while still trading energy with the pass every substep. The
/// loudest substep past [`QUIET_FROM`] reads 5.4e-7 a vertex, a little over a
/// quarter of this, and the test prints the line it falls on.
const QUIET_ENERGY_PER_VERT: f32 = 2.0e-6;

/// Triangle area cap the crossings are counted at.
///
/// Ten times the engine's own, which puts the trouser front near fifteen
/// hundred vertices instead of ten thousand. Counting crossings is quadratic
/// in the mesh and the scene is run twice all the way to the ground; at the
/// shipping density that is a quarter of an hour of a release test. The folds
/// are the same folds, made of fewer and larger triangles — and what the pass
/// costs is read separately, on the mesh the engine really meshes.
const COARSE: f64 = 2.0e-4;

/// The reference adult body the Anny goldens are taken against.
fn reference() -> BodyMesh {
    let phenotype = Phenotype {
        gender: 0.0,
        age: 0.8,
        muscle: 0.5,
        weight: 0.5,
        height: 0.5,
        proportions: 0.5,
    };
    body_mesh(&phenotype, &[0.0; 20])
}

/// The shipped trouser front, meshed at `max_area`, or at the engine's own
/// cap when that is zero.
fn panel(max_area: f64) -> ShapePipeline {
    let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
    let piece = draft
        .doc()
        .piece_named(block::FRONT)
        .expect("the block draws one piece");
    let contour = draft.outline_m(piece).to_vec();
    let (samples, shipped) = couture::for_contour(&contour);
    let cap = if max_area > 0.0 { max_area } else { shipped };
    ShapePipeline::build(&contour, samples, cap).expect("the block meshes")
}

/// What one whole drape to the ground left behind.
struct Drape {
    /// Most edges ever found through a triangle they share no vertex with.
    worst: usize,
    /// How many were left at the end.
    last: usize,
    /// Lowest and highest particle, in metres.
    span: (f32, f32),
    /// Contacts the pass parted, or zero when there was no pass.
    contacts: u64,
}

/// Lets the panel go over the body, onto the ground under it, and watches.
fn drape(pipe: &ShapePipeline, body: &Collider, sdf: &SdfGrid, mut held: Option<Layers>) -> Drape {
    let cons = pipe.constraints(COMPLIANCE);
    let seams = Seams::default();
    let floor = body.ground().map_or(Floor::none(), Floor::at);
    let stage = Stage::around(sdf).on(floor);
    let mut state = couture::drop_state(pipe, body.release_height());

    let (mut worst, mut last) = (0usize, 0usize);
    for _ in 0..STEPS / WATCH {
        for _ in 0..WATCH {
            let on = held.as_mut();
            xpbd::substep(&mut state, &cons, &seams, &stage, on, DT);
        }
        last = xpbd::self_crossings(&state, &pipe.tris);
        worst = worst.max(last);
    }
    Drape {
        worst,
        last,
        span: span(&state),
        contacts: held.map_or(0, |layers| layers.contacts()),
    }
}

/// Milliseconds a substep costs at the engine's own density, loose then held,
/// with the mesh that bought the number.
///
/// Timed from the heap and not from the release. A panel still flat in the air
/// has no layer anywhere near another, and timing the pass there would price
/// the broadphase on its own and call it the cost of self-collision.
fn cost(body: &Collider, sdf: &SdfGrid) -> (f64, f64, usize, usize) {
    let pipe = panel(0.0);
    let cons = pipe.constraints(COMPLIANCE);
    let seams = Seams::default();
    let floor = body.ground().map_or(Floor::none(), Floor::at);
    let stage = Stage::around(sdf).on(floor);
    let mut state = couture::drop_state(&pipe, body.release_height());
    for _ in 0..STEPS {
        xpbd::substep(&mut state, &cons, &seams, &stage, None, DT);
    }

    let started = Instant::now();
    for _ in 0..TIMED {
        xpbd::substep(&mut state, &cons, &seams, &stage, None, DT);
    }
    let loose = started.elapsed().as_secs_f64() * 1000.0 / TIMED as f64;

    let mut layers = Layers::of(&pipe.tris, &cons, &seams, pipe.pos2d.len());
    let started = Instant::now();
    for _ in 0..TIMED {
        let on = Some(&mut layers);
        xpbd::substep(&mut state, &cons, &seams, &stage, on, DT);
    }
    let held = started.elapsed().as_secs_f64() * 1000.0 / TIMED as f64;
    (loose, held, pipe.pos2d.len(), pipe.tris.len() / 3)
}

/// The cloth's lowest and highest particle, in metres.
fn span(state: &State) -> (f32, f32) {
    state
        .py
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), &y| (lo.min(y), hi.max(y)))
}

/// A garment that has come to rest on the ground is a garment in folds, and
/// folds are where cloth goes through itself.
///
/// The whole drape is run twice over the same body and the same ground, once
/// with the pass and once without, and the crossings are counted through both.
/// The count without it is reported beside the count with it, because a number
/// that was already zero proves nothing about a pass.
///
/// What it costs is printed rather than asserted. A budget written into a test
/// is a budget that passes on the machine it was written on.
#[test]
#[ignore = "release-only: a real body baked and three whole drapes run"]
fn a_garment_settled_on_the_ground_stops_going_through_itself() {
    let mesh = reference();
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    let sdf = bake::sdf(&mesh).expect("the Anny body is closed and orientable");
    let ground = body.ground().expect("a baked body stands on a plane");
    let pipe = panel(COARSE);
    let cons = pipe.constraints(COMPLIANCE);
    let layers = Layers::of(&pipe.tris, &cons, &Seams::default(), pipe.pos2d.len());
    println!(
        "counted on {} vertices · {} triangles · thickness {:.5} m · ground {ground:.4}",
        pipe.pos2d.len(),
        pipe.tris.len() / 3,
        layers.thickness()
    );

    let loose = drape(&pipe, &body, &sdf, None);
    let kept = drape(&pipe, &body, &sdf, Some(layers));
    report("loose", &loose);
    report("held ", &kept);

    let (without, with, verts, tris) = cost(&body, &sdf);
    println!(
        "cost on the shipped mesh, {verts} vertices and {tris} triangles, timed from the heap: \
         {without:.3} ms a substep loose, {with:.3} ms held · {:.0} against {:.0} substeps a \
         second · {:.1}x · the sim thread asks for 600",
        1000.0 / without,
        1000.0 / with,
        with / without
    );

    assert!(
        loose.worst > 0,
        "the heap has to go through itself without the pass, or this is no test"
    );
    assert!(
        kept.worst * 4 < loose.worst,
        "the pass has to take most of them away: {} against {}",
        kept.worst,
        loose.worst
    );
    assert!(
        kept.span.0 >= ground,
        "and nothing went through the ground: {} under {ground}",
        kept.span.0
    );
}

/// Holding a heap apart must not be what keeps it awake.
///
/// The same heap, run the way the sim thread runs it: the kinetic damper after
/// every substep, and the energy it hands back read against
/// [`QUIET_ENERGY_PER_VERT`]. Every substep is read and the loudest one is what
/// is judged, because a mean lets a heap that twitches once a second pass, and
/// a reading taken once a tick can land on one of the damper's zeroes.
#[test]
#[ignore = "release-only: a real body baked and a hundred seconds of heap"]
fn a_heap_held_apart_goes_quiet_and_stays_quiet() {
    let mesh = reference();
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    let sdf = bake::sdf(&mesh).expect("the Anny body is closed and orientable");
    let pipe = panel(COARSE);
    let cons = pipe.constraints(COMPLIANCE);
    let seams = Seams::default();
    let mut layers = Layers::of(&pipe.tris, &cons, &seams, pipe.pos2d.len());
    let floor = body.ground().map_or(Floor::none(), Floor::at);
    let stage = Stage::around(&sdf).on(floor);
    let mut state = couture::drop_state(&pipe, body.release_height());
    let mut damper = KineticDamper::new();
    let per_vertex = 1.0 / state.len() as f32;

    let (mut loudest, mut fastest, mut late) = (0.0f32, 0.0f32, 0.0f32);
    for step in 1..=WATCHED {
        let on = Some(&mut layers);
        xpbd::substep(&mut state, &cons, &seams, &stage, on, DT);
        fastest = fastest.max(xpbd::max_speed(&state));
        let energy = damper.observe(&mut state) * per_vertex;
        loudest = loudest.max(energy);
        if step > QUIET_FROM {
            late = late.max(energy);
        }
        if step % LINE == 0 {
            println!(
                "to {step}: loudest {loudest:.3e} a vertex · fastest {fastest:.4} m/s · {} crossings",
                xpbd::self_crossings(&state, &pipe.tris)
            );
            (loudest, fastest) = (0.0, 0.0);
        }
    }
    assert!(
        late < QUIET_ENERGY_PER_VERT,
        "the heap never went quiet and stayed quiet: {late:e} after {QUIET_FROM}"
    );
}

fn report(name: &str, drape: &Drape) {
    println!(
        "{name}: worst {} crossings, {} left at rest · cloth {:.4}..{:.4} · {} contacts parted",
        drape.worst, drape.last, drape.span.0, drape.span.1, drape.contacts
    );
}
