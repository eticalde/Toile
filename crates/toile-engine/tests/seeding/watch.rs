use std::time::{Duration, Instant};

use toile_engine::body::{BodyMesh, Phenotype, bake, body_mesh};
use toile_engine::session::Session;
use toile_sim::xpbd::SdfGrid;

/// How long a test waits on the sim thread before calling it stuck.
///
/// The thread runs at the wall clock's own cadence — ten substeps every
/// sixteen milliseconds — so the slowest scene here is about thirty-five
/// seconds of real time, and a bake stands in front of it.
pub const PATIENCE: Duration = Duration::from_secs(180);

/// Where in the drape a garment still being worn is measured, in substeps.
///
/// Three simulated seconds: down onto the body, and long before it has slid
/// off one. A scene that is asked how a garment sits *on* the body has to be
/// read here and not at rest, because at rest it is on the ground.
pub const MARK: u64 = 1800;

/// Substeps a drape is given to come to rest in.
///
/// The trouser front is the slowest of these scenes, quiet at 20,780; this
/// leaves half as much again. Counted in substeps and not in seconds, so the
/// budget means the same thing on a fast machine and a slow one.
pub const REST: u64 = 30_000;

/// How far the garment must have fallen to count as landed, in metres.
///
/// It is released a clearance above the body's top, so a fraction of that
/// clearance tells a panel that came down from one still hanging where it
/// was let go.
pub const LANDED: f32 = 0.05;

/// The reference adult body the Anny goldens are taken against.
pub fn reference() -> BodyMesh {
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
/// particles a fraction of a millimetre under the skin. Counting those would
/// call a good drape swallowed. What must never happen is a particle out past
/// the band, where the field is saturated flat, its gradient is exactly zero,
/// and no contact solve has a normal to push it back out along.
pub fn buried(sdf: &SdfGrid, points: &[[f32; 3]]) -> (usize, usize) {
    let floor = -(bake::BAND as f32);
    let deep = points
        .iter()
        .filter(|p| sdf.sample(p[0], p[1], p[2]) <= floor)
        .count();
    (deep, points.len())
}

/// Particles close enough to the skin to be held there by the contact solve.
pub fn touching(sdf: &SdfGrid, points: &[[f32; 3]]) -> usize {
    let cell = bake::CELL as f32;
    points
        .iter()
        .filter(|p| sdf.sample(p[0], p[1], p[2]).abs() <= cell)
        .count()
}

/// The cloth's lowest and highest particle, in metres.
pub fn span(points: &[[f32; 3]]) -> (f32, f32) {
    points.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
        (lo.min(p[1]), hi.max(p[1]))
    })
}

/// Where the cloth's weight lies horizontally, as x and z in metres.
///
/// Reading a drape at rest needs this, because "on the body" stopped meaning
/// "touching the skin" the moment the floor landed: nothing holds a garment up
/// yet, so it comes to rest in a heap around the feet and grazes the skin
/// almost by accident — the startup bodice with sixteen of its twelve thousand
/// particles. Where that heap sits still tells a garment that came down this
/// body from one that came down beside it.
pub fn footing(points: &[[f32; 3]]) -> (f32, f32) {
    let n = points.len() as f32;
    let (mut x, mut z) = (0.0f32, 0.0f32);
    for p in points {
        x += p[0];
        z += p[2];
    }
    (x / n, z / n)
}

/// The positions the latest snapshot carries.
fn points_of(session: &Session) -> Vec<[f32; 3]> {
    session.snapshot().positions.as_chunks::<3>().0.to_vec()
}

/// The frame after the sim thread has run [`MARK`] substeps.
///
/// # Panics
/// If the sim thread never reaches [`MARK`] substeps.
pub fn at_mark(session: &Session) -> Vec<[f32; 3]> {
    let deadline = Instant::now() + PATIENCE;
    while Instant::now() < deadline && session.snapshot().substeps < MARK {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        session.snapshot().substeps >= MARK,
        "the sim thread never reached {MARK} substeps"
    );
    points_of(session)
}

/// The frame the drape goes quiet on, and the substep it happened at.
///
/// # Panics
/// If the drape never goes quiet inside [`REST`] substeps, which since the
/// floor landed means something is keeping it moving rather than that there
/// is nothing to come to rest on.
pub fn at_rest(session: &Session) -> (Vec<[f32; 3]>, u64) {
    let deadline = Instant::now() + PATIENCE;
    while Instant::now() < deadline {
        if session.settled() {
            return (points_of(session), session.snapshot().substeps);
        }
        assert!(
            session.snapshot().substeps < REST,
            "the drape was still moving after {REST} substeps"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("the sim thread never settled inside {PATIENCE:?}");
}

/// Whether the drape goes to sleep inside `budget` substeps.
///
/// # Panics
/// If the sim thread never runs that many substeps, which means it is stuck
/// rather than busy.
pub fn parks_by(session: &Session, budget: u64) -> bool {
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

/// What watching a whole drape found.
pub struct Watched {
    /// The most particles ever past the band, and the substep it happened at.
    pub worst: (usize, u64),
    /// The most of the garment ever within a cell of the skin, and when.
    pub worn: (usize, u64),
}

/// Watches every frame the sim publishes up to [`MARK`].
///
/// A sewn tube is let go round a limb with nothing holding it on, so it
/// slides down the leg and by the mark it is around the ankles. What must
/// hold for one is that no particle was ever driven past the band — at any
/// moment of the drape, not at one chosen instant.
///
/// # Panics
/// If the sim thread never reaches [`MARK`] substeps.
pub fn through_the_drape(session: &Session, sdf: &SdfGrid) -> Watched {
    let deadline = Instant::now() + PATIENCE;
    let mut seen = 0;
    let mut found = Watched {
        worst: (0, 0),
        worn: (0, 0),
    };
    while Instant::now() < deadline {
        let snap = session.snapshot();
        if snap.substeps > seen {
            seen = snap.substeps;
            let points = snap.positions.as_chunks::<3>().0;
            let (deep, _) = buried(sdf, points);
            if deep > found.worst.0 {
                found.worst = (deep, seen);
            }
            let on_skin = touching(sdf, points);
            if on_skin > found.worn.0 {
                found.worn = (on_skin, seen);
            }
        }
        if seen >= MARK {
            return found;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("the sim thread never reached {MARK} substeps");
}
