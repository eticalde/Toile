use toile_engine::body::{Collider, bake};
use toile_engine::couture::HOLDS_ITS_RATIO;
use toile_engine::session::Session;
use toile_sim::xpbd::SdfGrid;

use crate::fit::shut;
use crate::skirt::skirt;
use crate::watch::{MARK, SETTLED, at_substep, band_of, reference, span, touching};

/// The ratio the waistband is held to: the length it was drawn at, less the
/// fifteen per cent a waistband is gathered by.
const GATHERED: f64 = 0.85;

/// A ratio that leaves the band wider than everything under it.
///
/// The reference body's hip is 103 cm and the skirt's waistline is 87 cm of
/// cloth, so at 1.6 the band asks for 139 cm and there is nothing on the way
/// down it has to stretch over. An elastic that holds anything holds nothing
/// true, and this is the arm that says so.
const TOO_LOOSE: f64 = 1.6;

/// Where the strength rail's knee was measured: from here up to the default
/// the band firms by a couple of points, and from here down by twenty.
const KNEE: f64 = 3.0;

/// How much higher the gripping band's cloth must end than a band that grips
/// nothing, in metres.
const HIGHER: f32 = 0.25;

/// What one drape of the skirt came to at a fixed substep.
struct Read {
    /// The ring it was let go on, in metres.
    ring: f32,
    /// How long the band measured, and what it is held to.
    band: (f64, f64),
    /// How high the band's own cloth stood.
    at: f64,
    /// Lowest and highest particle.
    cloth: (f32, f32),
    /// Particles within a cell of the skin, and particles in all.
    skin: (usize, usize),
}

/// Drapes the skirt with the band it is given and reads it at `substeps`.
fn wear(scene: &str, body: &Collider, sdf: &SdfGrid, band: Option<(f64, f64)>) -> (Read, Read) {
    let session = Session::from_doc(skirt(band), body.clone()).expect("the skirt drapes");
    assert!(
        session.seam_faults().is_empty(),
        "{scene}: the skirt's seams all pair: {:?}",
        session.seam_faults()
    );
    let ring = session.layout().expect("the seams place the skirt").stand;
    let held = session.held_cloth();
    let read = |substeps: u64| {
        let points = at_substep(&session, substeps);
        let (band, at) = band_of(&held, &points);
        // Held tight, held loose and not held at all, at both reads: the band
        // decides where the garment ends up and never whether its seams met.
        // The widest sewn pair over these six drapes stands 0.2 mm apart.
        shut(
            scene,
            &format!("at {substeps}"),
            &session.sewn_pairs(),
            &points,
        );
        Read {
            ring,
            band,
            at,
            cloth: span(&points),
            skin: (touching(sdf, &points), points.len()),
        }
    };
    let (early, late) = (read(MARK), read(SETTLED));
    for (when, it) in [(MARK, &early), (SETTLED, &late)] {
        println!(
            "{scene} at {when}: band {:.4} m of {:.4} held ({:.0}%) at y {:.4} \
             · cloth {:.4}..{:.4} · {} of {} on the skin · ring {ring:.4}",
            it.band.0,
            it.band.1,
            100.0 * it.band.0 / it.band.1.max(1.0e-9),
            it.at,
            it.cloth.0,
            it.cloth.1,
            it.skin.0,
            it.skin.1,
        );
    }
    (early, late)
}

/// What a waistband is worth on a body, measured against the same skirt with
/// nothing holding it and against one held too loose to grip.
///
/// What the band buys: it places the garment at the body's own waist rather
/// than at its chest, it holds the cloth it covers to within a quarter of the
/// length it is held to instead of being dragged half again as long, and under
/// a contact that reads the push it leaves the garment standing 0.375 m higher.
///
/// What it does not buy is a garment worn where it was put. The band grips the
/// waist inside 150 substeps, then travels down the legs keeping its own girth
/// until the body is that size again. Sweeping harder is a price and not an
/// answer: 256 sweeps end down the legs as 64 do, and only 1024 — three sweeps
/// of the whole garment a substep — was still above the hip at [`SETTLED`]. Nor
/// can Coulomb: the push a band this stiff makes in a substep is under a
/// millimetre, skin takes half of it, and the cloth below asks for three. What
/// does hold it is the anchor, and this scene is its control: `hang.rs` hangs
/// the same skirt from the waist ring and it stays there.
#[test]
#[ignore = "release-only: a real body baked and three whole drapes run"]
fn a_waistband_grips_the_waist_and_still_travels_down_the_body() {
    let mesh = reference();
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    let sdf = bake::sdf(&mesh).expect("the Anny body is closed and orientable");
    let floor = body.ground().expect("a baked body stands on a plane");

    let (held, held_late) = wear(
        "held at 85 %",
        &body,
        &sdf,
        Some((GATHERED, HOLDS_ITS_RATIO)),
    );
    let (loose, loose_late) = wear(
        "held at 160 %",
        &body,
        &sdf,
        Some((TOO_LOOSE, HOLDS_ITS_RATIO)),
    );
    let (bare, bare_late) = wear("no band at all", &body, &sdf, None);

    assert!(
        held.ring < bare.ring - 0.10,
        "the band hangs the skirt at the waist while the widest girth hangs it \
         at the chest, a hand's breadth higher: {} against {}",
        held.ring,
        bare.ring
    );
    // The one reading on a garment that the extra sweeps over held edges own,
    // and the assertion that fails without them: swept sixty-four times the
    // band stands at 120 % of its rest here, and never swept again at 149 %.
    assert!(
        held.band.0 < held.band.1 * 1.25,
        "the band is holding its ratio rather than being dragged open, which \
         is what sweeping the held edges again buys: {:.4} m of {:.4}",
        held.band.0,
        held.band.1
    );
    assert!(
        loose.band.0 < loose.band.1,
        "and one asked for more cloth than there is pulls on nothing: {:.4} m \
         of {:.4}",
        loose.band.0,
        loose.band.1
    );
    assert!(
        held.at > loose.at + 0.10,
        "at the mark the gripping band is still up on the body and the loose \
         one has gone: {:.4} against {:.4}",
        held.at,
        loose.at
    );
    // All three reach the ground, and the band's worth is how far up the body
    // it holds the cloth standing there: 0.57 m of it against 0.19 m. When a
    // garment stays on the body outright, the first of these is the assertion
    // to invert.
    for (scene, it) in [
        ("held at 85 %", &held_late),
        ("held at 160 %", &loose_late),
        ("no band at all", &bare_late),
    ] {
        assert!(
            (it.cloth.0 - floor).abs() < 1.0e-3,
            "{scene}: the skirt came down to the ground: {} against {floor}",
            it.cloth.0
        );
    }
    for (scene, it) in [("held at 160 %", &loose_late), ("no band", &bare_late)] {
        assert!(
            held_late.cloth.1 > it.cloth.1 + HIGHER,
            "the gripping band left the cloth standing far higher than {scene} \
             did: {} against {}",
            held_late.cloth.1,
            it.cloth.1
        );
    }
}

/// The strength rail reads differently along its length, on a whole garment.
///
/// Three drapes of the same skirt: the slack end of what the interface
/// offers, its default, and the knee between them. Measured at the mark, the
/// default band stands at 120 % of the length it is held to and its cloth is
/// 10.6 cm higher up the body than the slack one's, which stands at 143 %; at
/// three it already stands at 122 %, so the rail does its work under the knee.
/// Those are the readings the Spanish beside the rail is written from: 20 %
/// over rest at the default against 43 % at a tenth, near enough the twice as
/// much give it promises.
///
/// What is read here is the compliance a strength is written at. The sweeps
/// held edges get are pinned by the quarter in
/// `a_waistband_grips_the_waist_and_still_travels_down_the_body`, though the
/// knee leans on them too: never swept again the three bands stand at 149 %,
/// 153 % and 165 %, and there is no knee to find.
#[test]
#[ignore = "release-only: a real body baked and three whole drapes run"]
fn the_settled_band_answers_the_strength_it_was_given() {
    let mesh = reference();
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    let sdf = bake::sdf(&mesh).expect("the Anny body is closed and orientable");

    let (firm, _) = wear(
        "strength 10",
        &body,
        &sdf,
        Some((GATHERED, HOLDS_ITS_RATIO)),
    );
    let (knee, _) = wear("strength 3", &body, &sdf, Some((GATHERED, KNEE)));
    let (slack, _) = wear("strength 0.1", &body, &sdf, Some((GATHERED, 0.1)));
    assert!(
        firm.band.0 < slack.band.0 - 0.05,
        "the firm band held its length and the slack one was drawn out: {:.4} m \
         against {:.4} m",
        firm.band.0,
        slack.band.0
    );
    // And the knee earns its name: more of the give is below it than above it.
    // Measured, 0.0566 m of the span sits between the firm band and the knee
    // and 0.0983 m between the knee and the slack one. It used to be nine
    // times as lopsided, and what flattened it is the release rather than
    // the strength: a band let go on a hoop its own length starts
    // unstretched, so a firm one has less to give back by the mark than it
    // had when every band was let go pulled out onto one ring sized for the
    // widest cloth in the garment.
    assert!(
        (knee.band.0 - firm.band.0) < 0.75 * (slack.band.0 - knee.band.0),
        "and most of that is over by the knee: {:.4} m there, between {:.4} m \
         and {:.4} m",
        knee.band.0,
        firm.band.0,
        slack.band.0
    );
    assert!(
        firm.at > slack.at + 0.05,
        "and it is still further up the body for it: {:.4} against {:.4}",
        firm.at,
        slack.at
    );
    assert!(
        firm.skin.0 > slack.skin.0,
        "with more of the garment against the skin: {} against {}",
        firm.skin.0,
        slack.skin.0
    );
}
