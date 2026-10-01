use toile_anny::phenotype::LEVERS;
use toile_engine::body::{BodyMesh, Collider, Phenotype, bake, body_mesh, measured_anny};
use toile_engine::couture::HOLDS_ITS_RATIO;
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
    hips_at(1.0)
}

/// The same body with its hips most of the way back towards the reference
/// adult's.
///
/// Not an edge of what the sliders draw but a point inside it, which is why it
/// is here: a person reaches it by typing a 105 cm hip against a 61 cm waist,
/// and it is the body the anchor's old cap broke on while both ends of the
/// range passed.
pub fn hips_a_little_wide() -> BodyMesh {
    hips_at(0.6)
}

/// The three-lever body with its hips wherever `hips` puts them.
fn hips_at(hips: f64) -> BodyMesh {
    let phenotype = Phenotype {
        gender: 1.0,
        age: 0.8,
        muscle: 0.5,
        weight: 0.5,
        height: 0.5,
        proportions: 0.5,
    };
    let mut levers = [0.0f64; 20];
    levers[lever("hips-circ")] = hips;
    levers[lever("thigh-circ")] = 1.0;
    levers[lever("waist-circ")] = -1.0;
    body_mesh(&phenotype, &levers)
}

/// A skirt cut for a body with a 50 cm waist-to-hip drop comes to rest on that
/// body and not inside it, and on this body it now closes.
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
fn a_skirt_cut_for_a_wide_hipped_body_rests_clear_of_it_and_closes() {
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
    meets(worst);
}

/// How far apart the two sides of a seam may stand at rest and still be a seam
/// that met, in metres.
///
/// Not the sewing's own `SEAM_SHUT`, which is what its closing phase stops
/// waiting at — that phase runs with gravity off and nothing else pulling, and
/// this reading is taken at rest with the garment's whole weight on the seam
/// and the body pushing back through it. Five millimetres is a seam a machinist
/// would sew; what is being asserted is that a side seam which stood 115.3 mm
/// open has met, not that it met to the tolerance of a phase that is long over.
pub const MET: f32 = 0.005;

/// The fit this body's skirt now has, recorded rather than described.
///
/// It did not have it. Let go on one hoop sized to clear the person, the
/// waistband landed on a hoop 1.29566 m round carrying 0.61189 m of cloth — 47
/// % coverage — and the seam rested 115.3 mm open, which this test asserted
/// that it did. Let go on a hoop per ordinate the band's own cloth lands on a
/// 0.61189 m hoop, and the seam rests 1.7 mm open with nothing past the band at
/// any published frame of the drape.
///
/// Two ways to fail, and both are news. A reading back up near a tenth of a
/// metre means the garment stopped being placed on hoops its own size. A
/// reading of exactly zero with particles past the band means the burial is
/// back — before the contact solve learned to retreat along an overshooting
/// step, this same seam shut to 0.2 mm by pulling its two sides through the
/// abdomen.
fn meets(worst: f32) {
    assert!(
        worst < MET,
        "the wide-hipped skirt's side seam rests {:.1} mm open, against the \
         {:.1} mm a met seam is held to",
        worst * 1000.0,
        MET * 1000.0
    );
}
