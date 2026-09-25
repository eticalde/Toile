use toile_anny::phenotype::LEVERS;
use toile_engine::body::{BodyMesh, Collider, Phenotype, bake, body_mesh, measured_anny};
use toile_engine::couture::HOLDS_ITS_RATIO;
use toile_engine::session::Session;

use crate::rests_clear_of_the_body;
use crate::skirt::{Cut, cut_to};
use crate::watch::through_the_drape;

/// The ratio a waistband is gathered to.
const GATHERED: f64 = 0.85;

/// Where a lever sits in the vector [`body_mesh`] takes, by its own label.
///
/// By label and not by number, because the vector's order is the catalogue's
/// and a test that hard-coded an index would go on draping a body nobody asked
/// for if that order ever changed.
///
/// # Panics
/// If the catalogue stops carrying the lever named.
fn lever(label: &str) -> usize {
    LEVERS
        .iter()
        .position(|&name| name == label)
        .unwrap_or_else(|| panic!("the lever catalogue carries `{label}`"))
}

/// A body at the far corner of the shape a person can draw with the sliders:
/// female, with the hips and thighs a notch wider and the waist a notch
/// narrower than the reference adult the rest of this suite runs.
///
/// Three of the twenty levers, each at one end of its own travel, which is a
/// body the studio hands out for the asking rather than a shape invented here.
/// What it is extreme in is the one thing a skirt has to cross: the drop from
/// the waist to the hip, which on this body is the better part of half a metre
/// and on the reference adult is under a third of that.
fn wide_hipped() -> BodyMesh {
    let phenotype = Phenotype {
        gender: 1.0,
        age: 0.8,
        muscle: 0.5,
        weight: 0.5,
        height: 0.5,
        proportions: 0.5,
    };
    let mut levers = [0.0f64; 20];
    levers[lever("hips-circ")] = 1.0;
    levers[lever("thigh-circ")] = 1.0;
    levers[lever("waist-circ")] = -1.0;
    body_mesh(&phenotype, &levers)
}

/// A skirt cut for a body with a 50 cm waist-to-hip drop comes to rest on that
/// body and not inside it.
///
/// The suite's other seeded scenes all run the one reference adult, and burial
/// is a thing a body's own shape decides: what puts cloth where the field can
/// no longer report a normal is a constraint reaching further in one substep
/// than the band the bake was taken to, and how far a seam has to reach is how
/// far apart the body holds the two sides of it. On the reference adult the
/// hip and the waist are close enough that the reach stays short. So one body
/// is one reading, and this is the second.
///
/// Read the whole way down and then at rest, because the two say different
/// things. The watch sees the frames the sim publishes, so what it catches is
/// cloth that stayed past the band long enough to be shown to somebody; at
/// rest is where a garment that has settled inside a person is caught, and
/// that is the sharper of the two, because out past the band the field is
/// saturated flat and nothing in the contact solve is looking for it.
#[test]
#[ignore = "release-only: an extreme body baked and a whole drape run"]
fn a_skirt_cut_for_a_wide_hipped_body_rests_clear_of_it() {
    let mesh = wide_hipped();
    let tape = measured_anny(&mesh);
    let of = |name: &str| {
        tape.get(name)
            .unwrap_or_else(|| panic!("the tape reports `{name}`"))
    };
    let (waist, hip) = (of("cintura"), of("cadera"));
    println!(
        "wide-hipped body: waist {waist:.1} cm · hip {hip:.1} cm · drop {:.1} cm · thigh {:.1} \
         cm · stature {:.1} cm",
        hip - waist,
        of("muslo"),
        of("estatura")
    );
    assert!(
        hip - waist > 40.0,
        "the sliders drew a body a skirt has to cross a hip to get onto: its \
         waist-to-hip drop is {:.1} cm",
        hip - waist
    );

    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    let sdf = bake::sdf(&mesh).expect("the Anny body is closed and orientable");
    let session = Session::from_doc(
        cut_to(Cut::WIDE_HIPPED, Some((GATHERED, HOLDS_ITS_RATIO))),
        body,
    )
    .expect("the skirt drapes");
    assert!(
        session.seam_faults().is_empty(),
        "the skirt's seams all pair: {:?}",
        session.seam_faults()
    );
    let watched = through_the_drape(&session, &sdf);
    println!(
        "wide-hipped drape: worst {} past the band at substep {} · {} on the skin at {}",
        watched.worst.0, watched.worst.1, watched.worn.0, watched.worn.1
    );
    assert_eq!(
        watched.worst.0, 0,
        "{} particles were driven past the band at substep {}",
        watched.worst.0, watched.worst.1
    );
    rests_clear_of_the_body("wide-hipped body", &session, &sdf);
    report_seams(&session);
}

/// Prints how far the sewn pairs still stand apart once the garment is at rest.
///
/// Read and not asserted, because what it reports is the fit and not a defect
/// of the solver: this cut has to carry 117 cm of hem past a 112 cm hip on a
/// waist of 61 cm, and it does not go. Before the contact solve learned to
/// retreat along an overshooting step the same seam closed to under a
/// millimetre — by pulling the two sides through the body — so a reading near
/// zero here would mean the burial is back and the test above missed it. A
/// number this test prints rather than judges is the only honest form of that:
/// nobody has decided what an open seam at rest is worth.
fn report_seams(session: &Session) {
    let pairs = session.sewn_pairs();
    let snap = session.snapshot();
    let at = |v: u32| {
        let i = v as usize * 3;
        [
            snap.positions[i],
            snap.positions[i + 1],
            snap.positions[i + 2],
        ]
    };
    let gap = |(a, b): (u32, u32)| {
        let (p, q) = (at(a), at(b));
        let d = [0, 1, 2].map(|k| p[k] - q[k]);
        (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
    };
    let worst = pairs.iter().copied().map(gap).fold(0.0f32, f32::max);
    let mean = pairs.iter().copied().map(gap).sum::<f32>() / pairs.len() as f32;
    println!(
        "wide-hipped seams: {} pairs at rest, worst {:.1} mm apart, mean {:.1} mm",
        pairs.len(),
        worst * 1000.0,
        mean * 1000.0
    );
}
