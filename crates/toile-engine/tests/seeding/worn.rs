use toile_engine::body::{Collider, bake};
use toile_engine::couture::HOLDS_ITS_RATIO;
use toile_engine::draft::Elastic;
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

/// How close to the ground a garment nothing holds lies once it has parked.
const ON_THE_GROUND: f32 = 0.20;

/// How far above the ground a garment something holds still stands.
const ON_THE_LEG: f32 = 0.40;

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

/// A waistband decides where a garment hangs, and now also how far down the
/// body it gets. Two scenes, the same skirt, differing by the band and nothing
/// else.
///
/// What the band buys first is the placement, outright. A garment is held up by
/// one line of itself, so that line is the one matched to a ring: with the band
/// the skirt hangs at the body's waist, 0.406 m; without it the rule has only
/// the widest girth to go on and stands the waistline at the chest, 0.585 m.
///
/// What it buys second, on the reference body this skirt is drafted to, is a
/// garment that stops on the legs. The band rests at 0.741 m, the body catches
/// it at 0.978 m round, and the drape goes quiet there, 0.375 m above where
/// the same skirt with nothing on it heaps on the floor. One body's reading
/// and no rule: on bodies it was not drafted to the band was worth far less.
///
/// What neither buys is a garment worn where it was put: `grip.rs` measures
/// the three arms of that and says what is left to do.
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
    // is against the skin at the mark. It then travels down at its own girth
    // until the body is that size again, and stops there. Both readings below
    // were the other way round before a contact read the push: the skirt with
    // the band ended on the floor too, and neither scene ever went quiet.
    assert!(
        held.late.1 - floor > ON_THE_LEG,
        "the banded skirt is still up on the body at {LONG}: its highest point \
         {} stands {} above the ground",
        held.late.1,
        held.late.1 - floor
    );
    assert!(
        bare.late.1 - floor < ON_THE_GROUND,
        "and the same skirt with nothing on it is heaped on the floor: its \
         highest point {} stands {} above the ground",
        bare.late.1,
        bare.late.1 - floor
    );
    assert!(
        held.rested.is_some(),
        "the banded skirt came to rest inside {REST} substeps: {:?}",
        held.rested
    );
}

/// The skirt's top edge at the mark, draped once with the band it is given.
fn top_at_mark(body: &Collider, band: Option<(f64, f64)>) -> (f32, f32) {
    let session = Session::from_doc(skirt(band), body.clone()).expect("the skirt drapes");
    let ring = session.layout().expect("the seams place the skirt");
    (ring.stand, span(&at_mark(&session)).1)
}

/// What the band is worth inside the solver, told apart from where it hung the
/// garment.
///
/// Two drapes of the same skirt, hung on the very same ring: a band's ratio is
/// no part of where a garment is placed — the placement reads the cloth the
/// band covers, never how far it pulls it in — so a waistband at the length it
/// was drawn hangs the skirt exactly where the gathered one does and holds it
/// nowhere. The difference left is the rest lengths, which is the one thing
/// nothing else in the tree could tell from their absence. Measured, it is
/// worth 73 mm of top edge at the mark: 0.273 m against 0.199 m.
#[test]
#[ignore = "release-only: a real body baked and two whole drapes run"]
fn the_band_the_solver_holds_carries_the_skirt_higher_than_the_ring_alone() {
    let mesh = reference();
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    let (ring, gathered) = top_at_mark(&body, Some((GATHERED, HOLDS_ITS_RATIO)));
    let (same, drawn) = top_at_mark(&body, Some((Elastic::NEUTRAL_RATIO, HOLDS_ITS_RATIO)));
    println!(
        "at {MARK}: top edge {gathered:.4} at {GATHERED} · {drawn:.4} at the drawn length \
         · rings {ring:.4} and {same:.4}"
    );
    assert!(
        (ring - same).abs() < 1.0e-6,
        "the two skirts hang from the same ring: {ring} and {same}"
    );
    assert!(
        gathered > drawn + 0.010,
        "the band held the skirt up while the same band at its drawn length let \
         it go: {gathered} against {drawn}"
    );
}
