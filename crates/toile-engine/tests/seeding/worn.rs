use toile_engine::body::{Collider, bake};
use toile_engine::couture::HOLDS_ITS_RATIO;
use toile_engine::session::Session;
use toile_sim::xpbd::SdfGrid;

use crate::skirt::skirt;
use crate::watch::{
    LONG, MARK, REST, at_mark, at_substep, buried, reference, rest_by, span, through_the_drape,
    touching,
};

/// The ratio the waistband is held to: the length it was drawn at, less the
/// fifteen per cent a waistband is gathered by.
const GATHERED: f64 = 0.85;

/// How close to the ground the whole garment lies once it has parked.
const ON_THE_GROUND: f32 = 0.20;

/// What one whole drape of the skirt came to.
struct Worn {
    /// How high the ring it was let go on stands, in metres.
    ring: f32,
    /// Lowest and highest particle at [`LONG`].
    late: (f32, f32),
    /// Where it declared itself at rest, when it did, and how high its top
    /// edge stood then.
    rested: Option<(u64, f32)>,
}

/// Drapes the skirt once, with the waistband it is given, and measures it.
fn wear(scene: &str, body: &Collider, sdf: &SdfGrid, band: Option<(f64, f64)>) -> Worn {
    let session = Session::from_doc(skirt(band), body.clone()).expect("the skirt drapes");
    assert!(
        session.seam_faults().is_empty(),
        "{scene}: the skirt's seams all pair: {:?}",
        session.seam_faults()
    );
    let ring = session.layout().expect("the seams place the skirt");
    // Watched before any mark is read, because it has to see the frames on the
    // way there: what must never happen at any moment is a particle driven out
    // past the band, where the field is flat and nothing carries it back.
    let watched = through_the_drape(&session, sdf);
    let worn = at_mark(&session);
    let (deep, all) = buried(sdf, &worn);
    println!(
        "{scene}: ring {:.3} m round at {:.4} · at {MARK} cloth {:.3}..{:.3}, \
         {} of {all} on the skin, {deep} buried · worst {} past the band at {}",
        ring.radius * std::f64::consts::TAU,
        ring.stand,
        span(&worn).0,
        span(&worn).1,
        touching(sdf, &worn),
        watched.worst.0,
        watched.worst.1,
    );
    assert_eq!(watched.worst.0, 0, "{scene}: driven past the band");
    assert_eq!(deep, 0, "{scene}: {deep} of {all} buried at the mark");

    let late = span(&at_substep(&session, LONG));
    let rested = rest_by(&session, REST).map(|(points, at)| (at, span(&points).1));
    println!(
        "{scene}: at {LONG} cloth {:.3}..{:.3} · {}",
        late.0,
        late.1,
        match rested {
            Some((at, high)) => format!("at rest by {at}, top edge {high:.3}"),
            None => format!("still moving at {REST}"),
        }
    );
    Worn {
        ring: ring.stand,
        late,
        rested,
    }
}

/// A waistband decides where a garment hangs — and, measured, does not yet
/// decide whether it stays there. Two scenes, the same skirt, differing by the
/// band and nothing else.
///
/// What the band buys is the placement, outright. A garment is held up by one
/// line of itself, so that line is the one matched to a ring: with the band
/// the skirt hangs at the body's waist, 0.406 m. Without it the rule has only
/// the widest girth to go on, matches the skirt's hip to the upper chest and
/// stands the waistline there, 0.585 m — two rings too high.
///
/// What it does not buy is a garment that stays on: see the last assertion.
#[test]
#[ignore = "release-only: a real body baked and two whole drapes run"]
fn a_waistband_decides_where_a_skirt_hangs() {
    let mesh = reference();
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    let sdf = bake::sdf(&mesh).expect("the Anny body is closed and orientable");
    let floor = body.ground().expect("a baked body stands on a plane");

    let held = wear(
        "waistband at 85%",
        &body,
        &sdf,
        Some((GATHERED, HOLDS_ITS_RATIO)),
    );
    let bare = wear("no waistband", &body, &sdf, None);

    assert!(
        held.ring < bare.ring - 0.10,
        "the band hangs the skirt at the waist while the widest girth hangs it \
         at the chest, a hand's breadth higher: {} against {}",
        held.ring,
        bare.ring
    );
    assert!(
        held.late.0 >= floor && bare.late.0 >= floor,
        "neither went through the ground: {} and {} under {floor}",
        held.late.0,
        bare.late.0
    );
    // The band comes in — weightless, it is pulled from the 1.21 m ring it was
    // let go on to the body's own waist inside 150 substeps, and half the cloth
    // is against the skin at the mark. Then it creeps down at about 30 mm a
    // second, and the rate does not move: not with the ratio (0.85, 0.70, 0.55
    // and 0.40 creep alike), not with the strength (10 and 100 alike), not with
    // the strain cap the constraint set already carries. That is a friction
    // reading the motion of a substep rather than the tension in the cloth,
    // which is the parameter Decision 12 says comes next.
    for (scene, worn) in [("with a band", &held), ("without one", &bare)] {
        assert!(
            worn.late.1 - floor < ON_THE_GROUND,
            "{scene} the skirt is no longer heaped on the floor by {LONG}: its \
             highest point {} stands {} above the ground. When a static \
             friction lets tension hold, this is the assertion to invert",
            worn.late.1,
            worn.late.1 - floor
        );
        if let Some((at, high)) = worn.rested {
            assert!(
                high - floor < ON_THE_GROUND,
                "{scene} it was still up at rest, {at} substeps in: {high}"
            );
        }
    }
}
