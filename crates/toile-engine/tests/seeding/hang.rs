use toile_engine::body::{Collider, bake};
use toile_engine::couture::HOLDS_ITS_RATIO;
use toile_engine::draft::Doc;
use toile_engine::session::Session;
use toile_engine::sync::Hanging;
use toile_sim::xpbd::SdfGrid;

use crate::skirt::{hang_from_the_waist, skirt};
use crate::watch::{
    REST, SETTLED, at_substep, buried, reference, rest_by, span, through_the_drape,
};

/// The same anchor on the bodies at and inside the edge of what the sliders
/// draw.
mod wide;

/// The ratio the reference skirt's waistband is gathered to, as `grip.rs`
/// writes it.
const GATHERED: f64 = 0.85;

/// What one drape came to: the cloth, and how far the hung run stands from
/// the ring it names.
struct Read {
    /// Lowest and highest particle, in metres.
    cloth: (f32, f32),
    /// Mean and worst signed distance from the hung run to its ring, in
    /// metres; `None` for a garment hung from nothing.
    off: Option<(f64, f64)>,
    /// What the published frame says about the same run, which is what the
    /// fitting room reads off it.
    told: Option<Hanging>,
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
    let told = session.snapshot().hanging;
    let (deep, all) = buried(sdf, &points);
    let read = Read {
        cloth: span(&points),
        off: off_its_ring(&hung, &points),
        told,
    };
    println!(
        "{scene} at {SETTLED}: {} vertices hung, {} · the frame says {} · cloth {:.4}..{:.4} · \
         {deep} of {all} buried",
        hung.len(),
        read.off.map_or_else(
            || "nothing held".to_owned(),
            |(mean, worst)| format!(
                "standing {:.1} mm off its ring, worst vertex {:.1} mm",
                mean * 1000.0,
                worst * 1000.0
            )
        ),
        read.told.map_or_else(
            || "nothing".to_owned(),
            |told| format!("{} runs, {:.1} mm off", told.runs, told.gap * 1000.0)
        ),
        read.cloth.0,
        read.cloth.1,
    );
    assert_eq!(
        deep, 0,
        "{scene}: {deep} of {all} particles are buried past the band"
    );
    // And how long it took, which is the one cost an anchor has: a run held at
    // a ring reaches its own rest by a different road than the same cloth
    // sliding down a leg, and neither of the two is the shorter one on both
    // bodies.
    println!(
        "{scene}: {}",
        match rest_by(&session, REST) {
            Some((_, at)) => format!("asleep by {at}"),
            None => format!("still moving at {REST}"),
        }
    );
    read
}

/// How far the hung cloth stands from the ring it names, in metres.
///
/// Signed, so a run that slid down the body reads negative and one the cloth
/// dragged up reads positive. Both the mean and the vertex furthest from the
/// ring, because a mean would pass a run that bowed a hand's breadth each way
/// in equal parts. `None` for a garment hung from nothing.
pub fn off_its_ring(hung: &[(u32, f32)], points: &[[f32; 3]]) -> Option<(f64, f64)> {
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
/// This is the complaint `grip.rs` records, answered: there the band grips the
/// waist and then travels down the legs keeping its own girth.
///
/// Both scenes and not one: a reading of the anchored drape alone says nothing,
/// and the control is the same document with the hang taken off. Measured, the
/// hung waistline settles 0.1 mm from the ring, its worst vertex 2.6 mm off,
/// and its cloth ends 1.24 m up where the control's reaches the ground.
///
/// What it costs: 20,900 substeps to sleep against the control's 13,520, where
/// before every height was let go on a hoop of its own cloth it was 33,500
/// against 13,860. Starting the right size round is less to be pulled into.
///
/// How far the run ends from its ring is a reading and not a bar, for the
/// reason written where it is read.
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
    assert_eq!(bare.told, None, "and its frame says nothing about hanging");
    // How far the run ends from its ring is a reading here and not a bar, and
    // the bar it replaced had no cause. `HANG_STEP` caps what one substep may
    // undo; nothing caps where the run settles, and `hang::solve` is not the
    // last phase of a substep, so the body and the floor move the vertex
    // again after the anchor lets go. Measured, the reference adult settles
    // 2.6 mm off its ring, the wide-hipped body 4.8 and the body between
    // them 8.9, against a dose of 5.0: the two that passed a cap of one
    // dose did so by coincidence. Two readings a simulated second and a
    // half apart do not earn a place either — with the anchor off entirely
    // the run reads the same at 12,000 substeps and at 20,000, because by
    // then it has finished sliding. What does fail when the anchor stops
    // working is the assertion at the end of this test.
    let (_, worst) = hung.off.expect("the hung skirt names a ring");
    // What the fitting room reads is this same cloth and the same quantity,
    // so the two agree to the tenth of a millimetre the bar prints. A cap's
    // worth of slack here is what let the two mean different things: read
    // where the anchor pulls, the frame said 1.6 mm of a run settling at 1.1.
    let told = hung.told.expect("the hung skirt publishes a reading");
    assert_eq!(told.runs, 2, "both halves of the skirt are hung");
    assert!(
        (f64::from(told.gap) - worst.abs()).abs() < 1.0e-4,
        "the frame reports {:.4} m off the ring where the cloth is {:.4} m off",
        told.gap,
        worst.abs()
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
