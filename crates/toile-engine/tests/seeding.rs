#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use std::time::{Duration, Instant};

use toile_engine::body::{BodyMesh, Collider, Phenotype, bake, body_mesh};
use toile_engine::draft::block;
use toile_engine::session::Session;
use toile_sim::xpbd::SdfGrid;

/// How long a test waits on the sim thread before calling it stuck.
const PATIENCE: Duration = Duration::from_secs(60);

/// Where in the drape the garment is measured, in substeps.
///
/// Three simulated seconds: after the panel has come down onto the body, and
/// seconds before it slides off one. There is no settled state to measure
/// instead, and the last test here is what says so.
const MARK: u64 = 1800;

/// Substeps a drape is given to go to sleep in.
///
/// The demo bodice parks on the sphere in a little over two thousand, and the
/// first half of the last test is what keeps this number honest: a budget too
/// short to catch the scene that does park would make the half about the body
/// pass against anything at all.
const PARKED: u64 = 3600;

/// How far the garment must have fallen to count as landed, in metres.
///
/// It is released a clearance above the body's top and comes to rest on it,
/// so the whole drop is that clearance; a fraction of it is enough to tell a
/// panel on the body from one still hanging where it was let go.
const LANDED: f32 = 0.05;

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

/// Particles buried past the band, and particles in all.
///
/// Not the field's bare sign, which is a razor edge: cloth resting on a body
/// straddles the zero isosurface, so every drape here reads several hundred
/// particles a fraction of a millimetre under the skin, and one settled on
/// the demo sphere reads nineteen hundred. Counting those would call a good
/// drape swallowed. What must never happen is a particle out past the band,
/// where the field is saturated flat, its gradient is exactly zero, and no
/// contact solve has a normal to push it back out along.
fn buried(sdf: &SdfGrid, points: &[[f32; 3]]) -> (usize, usize) {
    let floor = -(bake::BAND as f32);
    let deep = points
        .iter()
        .filter(|p| sdf.sample(p[0], p[1], p[2]) <= floor)
        .count();
    (deep, points.len())
}

/// Particles close enough to the skin to be held there by the contact solve.
///
/// One cell is the field's own resolution, and the gap a garment hangs at is
/// several of them, so this counts the cloth that is on the body rather than
/// near it.
fn touching(sdf: &SdfGrid, points: &[[f32; 3]]) -> usize {
    let cell = bake::CELL as f32;
    points
        .iter()
        .filter(|p| sdf.sample(p[0], p[1], p[2]).abs() <= cell)
        .count()
}

/// The cloth's lowest and highest particle, in metres.
fn span(points: &[[f32; 3]]) -> (f32, f32) {
    points.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
        (lo.min(p[1]), hi.max(p[1]))
    })
}

/// The frame after the sim thread has run [`MARK`] substeps.
///
/// Counted in substeps and not in seconds, because the solver's step is
/// fixed: the frame this returns is the same drape on a fast machine and on
/// a slow one.
fn at_mark(session: &Session) -> Vec<[f32; 3]> {
    let deadline = Instant::now() + PATIENCE;
    while Instant::now() < deadline && session.snapshot().substeps < MARK {
        std::thread::sleep(Duration::from_millis(2));
    }
    let snap = session.snapshot();
    assert!(
        snap.substeps >= MARK,
        "the sim thread never reached {MARK} substeps"
    );
    snap.positions.as_chunks::<3>().0.to_vec()
}

/// Whether the drape goes to sleep inside `budget` substeps.
///
/// # Panics
/// If the sim thread never runs that many substeps, which means it is stuck
/// rather than busy.
fn parks_by(session: &Session, budget: u64) -> bool {
    let deadline = Instant::now() + PATIENCE;
    while Instant::now() < deadline {
        if session.settled() {
            return true;
        }
        if session.snapshot().substeps >= budget {
            return false;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("the sim thread never reached {budget} substeps");
}

/// What a seeded scene is held to: the garment came down onto the body, it is
/// resting on it, and none of it is buried where the field cannot push it out
/// again.
///
/// Contact is the one that refuses a miss. A span that lies between the crown
/// and the feet is also where a panel that has slid off and is falling past
/// the hips reads: over this body the demo bodice keeps such a span for
/// seconds after the last of it has left the skin, so the span alone passes
/// in the very case this exists to refuse.
fn lands_on_the_body(scene: &str, session: &Session, sdf: &SdfGrid) {
    let (lo, hi) = session.collider().extent();
    let release = session.collider().release_height();
    let points = at_mark(session);
    let (low, high) = span(&points);
    let (deep, all) = buried(sdf, &points);
    let on_skin = touching(sdf, &points);
    println!(
        "{scene}: cloth {low:.3}..{high:.3}, body {:.3}..{:.3}, released {release:.3}, \
         {on_skin} of {all} on the skin, {deep} buried",
        lo[1], hi[1]
    );
    assert!(
        high < release - LANDED,
        "{scene}: the garment never came down from {release}: {high}"
    );
    assert!(
        low > lo[1],
        "{scene}: the garment fell past the body instead of onto it: {low} under {}",
        lo[1]
    );
    assert!(
        on_skin > 0,
        "{scene}: the garment is not on the body: not one of {all} particles \
         lies within a cell of the skin"
    );
    assert_eq!(
        deep, 0,
        "{scene}: {deep} of {all} particles are buried past the band"
    );
}

/// Opening a document lets its garment go over the body the document names,
/// and none of it ends up buried under that body's skin.
///
/// The failure this pins is not a near miss. A session seeded at some other
/// body's height starts a whole panel inside a person: let go at the sphere's
/// height over an adult body, 1,299 of the trouser front's 10,513 particles
/// begin out past the band, where the field is saturated flat and its
/// gradient is exactly zero. The contact solve has no normal to push along,
/// so they never come out — a minute later the panel has stopped moving with
/// sixty-three of them still in there.
#[test]
#[ignore = "release-only: a real body baked and a whole drape run"]
fn opening_a_document_does_not_bury_the_garment_in_the_body() {
    let mesh = reference();
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    // The field a second time: a `Collider` answers whether a point is under
    // the skin, and what is measured here is how far under, which only the
    // grid itself carries.
    let sdf = bake::sdf(&mesh).expect("the Anny body is closed and orientable");
    let session = Session::from_doc(block::trouser_front(), body).expect("the block drapes");
    lands_on_the_body("opened document", &session, &sdf);
}

/// The scene the app starts on, measured the same way, which is what said the
/// numbers above belonged to opening a document rather than to the body.
#[test]
#[ignore = "release-only: a real body baked and a whole drape run"]
fn the_startup_scene_does_not_bury_the_garment_in_the_body() {
    let mesh = reference();
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    let sdf = bake::sdf(&mesh).expect("the Anny body is closed and orientable");
    let session = Session::demo_bodice_over(body);
    lands_on_the_body("startup scene", &session, &sdf);
}

/// No drape over a body settles yet, which is why the two tests above measure
/// at a fixed substep instead of waiting for one that settles.
///
/// There is no floor. A panel is let go a clearance above the body's top,
/// which over an adult puts it over the crown of the head; it comes down
/// there, hangs while friction loses to gravity, slides off, and then falls
/// for as long as anything integrates it — hundreds of metres down within the
/// minute, and metres a second all the way. The sim thread sleeps on mean
/// kinetic energy per vertex, and a garment still falling never goes quiet.
///
/// So the same panel is run over both, to the same budget. The sphere half is
/// not scenery: a test that only ever asserts a negative cannot tell "this
/// never settles" from "nothing here would have noticed if it had", and the
/// scene that does park is what rules the second reading out.
#[test]
#[ignore = "release-only: a real body baked and a whole drape run"]
fn the_same_panel_parks_on_the_sphere_and_never_on_a_body() {
    assert!(
        parks_by(&Session::demo_bodice(), PARKED),
        "the demo bodice stopped parking on the sphere inside {PARKED} substeps, \
         so the budget no longer tells a drape that settles from one that never \
         will, and the half below proves nothing until it is raised"
    );

    let mesh = reference();
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    assert!(
        !parks_by(&Session::demo_bodice_over(body), PARKED),
        "a drape settled on a body: either a floor has landed or the panel now \
         stays on, and the two tests above should go back to measuring at rest"
    );
}
