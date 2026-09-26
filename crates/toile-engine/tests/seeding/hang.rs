use toile_engine::body::{Collider, bake};
use toile_engine::couture::{HANG_STEP, HOLDS_ITS_RATIO, SEAM_SHUT};
use toile_engine::draft::Doc;
use toile_engine::session::Session;
use toile_sim::xpbd::SdfGrid;

use crate::extremes::{GATHERED as WIDE_GATHERED, wide_hipped};
use crate::rests_clear_of_the_body;
use crate::skirt::{Cut, cut_to, hang_from_the_waist, skirt};
use crate::watch::{SETTLED, at_substep, buried, reference, span, through_the_drape};

/// The ratio the reference skirt's waistband is gathered to, as `grip.rs`
/// writes it.
const GATHERED: f64 = 0.85;

/// How far the hung cloth may stand from the ring it is held at before the
/// anchor is not holding it, in metres.
///
/// The anchor's own step and no threshold of this suite's: whatever the
/// stretch and the seams did to the run inside a substep, the anchor may undo
/// at most [`HANG_STEP`] of it before the substep ends, so one step is as
/// tight as an honest reading gets. Measured, the run settles well inside it.
const AT_ITS_RING: f32 = HANG_STEP;

/// What one drape came to: the cloth, and how far the hung run stands from
/// the ring it names.
struct Read {
    /// Lowest and highest particle, in metres.
    cloth: (f32, f32),
    /// Mean and worst signed distance from the hung run to its ring, in
    /// metres; `None` for a garment hung from nothing.
    off: Option<(f64, f64)>,
}

/// Drapes `doc` over `body` and reads it at [`SETTLED`].
///
/// Nothing may be past the band, and the reading is taken twice for the reason
/// `extremes.rs` takes it twice: a garment held on by being held inside the
/// person is the fault decision 17 was written to end, and an anchor that
/// bought its place that way would buy it in the substeps the contact solve
/// then undoes, where only the watch can see it.
fn wear(scene: &str, doc: Doc, body: &Collider, sdf: &SdfGrid) -> Read {
    let session = Session::from_doc(doc, body.clone()).expect("the skirt drapes");
    assert!(
        session.seam_faults().is_empty(),
        "{scene}: the skirt's seams all pair: {:?}",
        session.seam_faults()
    );
    let hung = session.hung_cloth();
    let watched = through_the_drape(&session, sdf);
    assert_eq!(
        watched.worst.0, 0,
        "{scene}: {} particles were driven past the band at substep {}",
        watched.worst.0, watched.worst.1
    );
    let points = at_substep(&session, SETTLED);
    let (deep, all) = buried(sdf, &points);
    let read = Read {
        cloth: span(&points),
        off: off_its_ring(&hung, &points),
    };
    println!(
        "{scene} at {SETTLED}: {} vertices hung, {} · cloth {:.4}..{:.4} · {deep} of {all} buried",
        hung.len(),
        read.off.map_or_else(
            || "nothing held".to_owned(),
            |(mean, worst)| format!(
                "standing {:.1} mm off its ring, worst vertex {:.1} mm",
                mean * 1000.0,
                worst * 1000.0
            )
        ),
        read.cloth.0,
        read.cloth.1,
    );
    assert_eq!(
        deep, 0,
        "{scene}: {deep} of {all} particles are buried past the band"
    );
    read
}

/// How far the hung cloth stands from the ring it names, in metres.
///
/// Signed, so a run that slid down the body reads negative and one the cloth
/// dragged up reads positive. Both the mean and the vertex furthest from the
/// ring, because the claim above is about the run and a mean would pass a run
/// that bowed a hand's breadth each way in equal parts. `None` for a garment
/// hung from nothing, which is a different answer from zero.
fn off_its_ring(hung: &[(u32, f32)], points: &[[f32; 3]]) -> Option<(f64, f64)> {
    let each = || {
        hung.iter()
            .map(|&(v, ring)| f64::from(points[v as usize][1] - ring))
    };
    let sum: f64 = each().sum();
    let worst = each().fold(
        0.0f64,
        |far, one| if one.abs() > far.abs() { one } else { far },
    );
    (!hung.is_empty()).then(|| (sum / hung.len() as f64, worst))
}

/// The reserve of decision 12, exercised: the same skirt, once hung from the
/// body's waist and once not.
///
/// This is the complaint `grip.rs` records, answered. There the band grips the
/// waist and then travels down the legs keeping its own girth, and neither
/// sweeping harder nor Coulomb stops it. Hung from the waist ring, the run
/// stays at that ring and the garment stays on the person.
///
/// Both scenes and not one, because a reading of the anchored drape alone says
/// nothing: the control is the same document with the hang taken off. Measured,
/// the hung waistline settles 0.1 mm from the ring and its highest cloth stands
/// 1.24 m above the ground; the control's reaches the ground and stands 0.57 m.
///
/// What it costs: this scene takes 33,500 substeps to sleep against the
/// control's 13,860 — 56 simulated seconds instead of 23 — while on the
/// wide-hipped body below it goes the other way, so it is the scene, not a tax.
#[test]
#[ignore = "release-only: a real body baked and two whole drapes run"]
fn a_skirt_hung_from_the_waist_stays_there_and_the_same_skirt_unhung_does_not() {
    let mesh = reference();
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    // The field a second time: a `Collider` answers whether a point is under
    // the skin, and what is counted here is how far under, which only the grid
    // itself carries.
    let sdf = bake::sdf(&mesh).expect("the Anny body is closed and orientable");

    let bare = wear(
        "nothing hung",
        skirt(Some((GATHERED, HOLDS_ITS_RATIO))),
        &body,
        &sdf,
    );
    let hung = wear(
        "hung from the waist",
        {
            let mut doc = skirt(Some((GATHERED, HOLDS_ITS_RATIO)));
            hang_from_the_waist(&mut doc);
            doc
        },
        &body,
        &sdf,
    );

    assert_eq!(bare.off, None, "the control hangs from nothing at all");
    let (mean, worst) = hung.off.expect("the hung skirt names a ring");
    assert!(
        worst.abs() < f64::from(AT_ITS_RING),
        "a vertex of the hung waistline came to rest {worst:.4} m from the ring \
         it names, with the run averaging {mean:.4} m"
    );
    // And the garment is on the body rather than round its feet. The control
    // is the measure: with the same band and nothing hung, its cloth ends far
    // lower. When something else holds the garment up, this is the assertion
    // that says so.
    assert!(
        hung.cloth.1 > bare.cloth.1 + 0.10,
        "the hung skirt stands higher than the one held by its band alone: \
         {:.4} against {:.4}",
        hung.cloth.1,
        bare.cloth.1
    );
}

/// And what the anchor does not buy: the wide-hipped skirt's side seam.
///
/// `extremes.rs` records that this seam stands open at rest and asserts that it
/// does. The anchor is the remedy decision 12 held in reserve, so it is the one
/// that has to be tried against it — and measured, it does not close it. What
/// separates that body from the reference is not where the garment is held but
/// that a seam pulls along the straight chord between its two ends and that
/// chord runs through the person, which no height a ring names can reach.
///
/// Two scenes again: the unanchored one open, the anchored one open too —
/// measured, 115.3 mm and 119.6 mm. The day something does close it, both of
/// these are the assertions to invert.
#[test]
#[ignore = "release-only: an extreme body baked and two whole drapes run"]
fn hanging_the_wide_hipped_skirt_from_the_waist_does_not_close_its_side_seam() {
    let mesh = wide_hipped();
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    let sdf = bake::sdf(&mesh).expect("the Anny body is closed and orientable");

    let bare = rests_open("unhung, wide-hipped", cut(false), &body, &sdf);
    let hung = rests_open("hung, wide-hipped", cut(true), &body, &sdf);
    assert!(
        hung > SEAM_SHUT && bare > SEAM_SHUT,
        "the wide-hipped side seam shut to {:.1} mm hung and {:.1} mm unhung. \
         If the anchor now closes it, these are the assertions to invert",
        hung * 1000.0,
        bare * 1000.0
    );
    // And what the anchor changed is small beside what it did not: the two
    // readings are the same gap, so the fault is not where the garment hangs.
    assert!(
        (hung - bare).abs() < 0.5 * bare,
        "hanging the garment moved the gap from {:.1} mm to {:.1} mm, which is \
         more than a body that does not fit can account for",
        bare * 1000.0,
        hung * 1000.0
    );
}

/// The wide-hipped skirt, hung from the waist or not.
fn cut(hung: bool) -> Doc {
    let mut doc = cut_to(Cut::WIDE_HIPPED, Some((WIDE_GATHERED, HOLDS_ITS_RATIO)));
    if hung {
        hang_from_the_waist(&mut doc);
    }
    doc
}

/// Drapes one wide-hipped scene, holds it to everything a seeded scene is held
/// to, and hands back how far its worst seam stands open at rest.
fn rests_open(scene: &str, doc: Doc, body: &Collider, sdf: &SdfGrid) -> f32 {
    let session = Session::from_doc(doc, body.clone()).expect("the skirt drapes");
    assert!(
        session.seam_faults().is_empty(),
        "{scene}: the skirt's seams all pair: {:?}",
        session.seam_faults()
    );
    let watched = through_the_drape(&session, sdf);
    assert_eq!(
        watched.worst.0, 0,
        "{scene}: {} particles were driven past the band at substep {}",
        watched.worst.0, watched.worst.1
    );
    let (worst, _) =
        rests_clear_of_the_body(scene, &session, sdf).expect("the skirt is sewn into a tube");
    worst
}
