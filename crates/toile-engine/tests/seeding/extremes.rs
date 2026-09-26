use toile_anny::phenotype::LEVERS;
use toile_engine::body::{BodyMesh, Collider, Phenotype, bake, body_mesh, measured_anny};
use toile_engine::couture::{HOLDS_ITS_RATIO, SEAM_SHUT};
use toile_engine::session::Session;

use crate::rests_clear_of_the_body;
use crate::skirt::{Cut, cut_to};
use crate::watch::through_the_drape;

/// The ratio a waistband is gathered to.
pub const GATHERED: f64 = 0.85;

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
pub fn wide_hipped() -> BodyMesh {
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

/// A skirt cut for a body with a 50 cm waist-to-hip drop comes to rest on
/// that body and not inside it, and on this body does not close.
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
fn a_skirt_cut_for_a_wide_hipped_body_rests_clear_of_it_and_does_not_close() {
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
    let seams = rests_clear_of_the_body("wide-hipped body", &session, &sdf);
    let (worst, _) = seams.expect("the skirt is sewn into a tube");
    still_open(worst);
}

/// The fit this body's skirt does not have, recorded rather than described.
///
/// What is asserted is that the seam did *not* shut, which needs no threshold
/// of this suite's own: [`SEAM_SHUT`] is the distance the solver's own closing
/// phase stops waiting at, so the negative of it is exactly as measured as the
/// positive. Nobody has decided what an open seam at rest is worth, and this
/// does not decide it either — it pins the state of the tree.
///
/// Two ways to fail, and both are news. A placement that hands the sewing a
/// gap it can close is the fix this waits on, from the family of decision 12:
/// the ring is sized from the widest cloth the product carries while the
/// product is stood at the ring its band belongs to. And a reading near zero
/// without such a change means the burial is back — before the contact solve
/// learned to retreat along an overshooting step, this same seam shut to
/// 0.2 mm by pulling its two sides through the abdomen.
fn still_open(worst: f32) {
    assert!(
        worst > SEAM_SHUT,
        "the wide-hipped skirt's side seam shut to {:.1} mm. If a placement \
         rule now closes it, this is the assertion to invert; if nothing was \
         placed differently, the cloth went through the body again",
        worst * 1000.0
    );
}
