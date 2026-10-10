use toile_engine::body::Collider;
use toile_engine::couture::Layout;
use toile_engine::session::Session;

use super::super::super::fit::{placed, report};
use super::super::super::watch::reference;
use super::super::Cut;
use super::{CUT, FRONTS, Faced, fronts, shirt};

/// How far a reading may stand from the centre front, in degrees.
///
/// Mesh and not rule, and the same slack the three-panel heading bench takes:
/// every reading is asked at an abscissa the pattern draws while the hoop it
/// is divided by is the cloth the mesher laid, so a line comes out a couple of
/// degrees round from where it was drawn. Measured on this garment the worst
/// declared reading is 1.56°, and what it is held against is half a turn.
const SLACK: f64 = 2.5;

/// What one release of the shirt came to.
struct Read {
    /// Each ring's centre front, in degrees from the body's own.
    turns: Vec<f64>,
    /// How high each ring stands, in metres.
    stands: Vec<f32>,
    /// Worst and mean seam gap at release, in metres.
    seams: (f32, f32),
}

/// Lets one shirt go over `body` and reads where each of its rings faces.
fn let_go(scene: &str, cut: Cut, faced: Faced, body: &Collider) -> Read {
    let session = Session::from_doc(shirt(cut, faced), body.clone()).expect("the shirt drapes");
    assert!(
        session.seam_faults().is_empty(),
        "{scene}: every seam pairs: {:?}",
        session.seam_faults()
    );
    assert_eq!(session.adrift(), 0, "{scene}: every piece has a place");
    let rings = session.rings();
    let at = fronts(cut);
    let read = Read {
        turns: FRONTS
            .iter()
            .zip(at)
            .map(|(&(piece, named), abscissa)| facing(&rings, piece, abscissa, named))
            .collect(),
        stands: rings.iter().map(|ring| ring.stand).collect(),
        seams: report(
            scene,
            "at release",
            &session.sewn_pairs(),
            &placed(&session.released()),
        )
        .expect("the shirt carries six seams"),
    };
    println!(
        "{scene}: cuerpo {:8.3}\u{b0} \u{b7} cuello {:8.3}\u{b0} \u{b7} pretina {:8.3}\u{b0} \
         \u{b7} alturas {:?}",
        read.turns[0], read.turns[1], read.turns[2], read.stands
    );
    read
}

/// Where one line of cloth came out, in degrees, on whichever ring carries it.
fn facing(rings: &[Layout], piece: usize, abscissa: f64, named: &str) -> f64 {
    rings
        .iter()
        .find_map(|ring| ring.round.facing(piece, abscissa, ring.crest))
        .unwrap_or_else(|| panic!("some ring of the shirt carries the {named}"))
}

/// How far a turn stands from the centre front, the short way round.
fn apart(turn: f64) -> f64 {
    let gap = turn.rem_euclid(360.0);
    gap.min(360.0 - gap)
}

/// One declared heading turns every ring of the shirt, and not only the ring
/// it was written on.
///
/// The figure of this part, and it is not the one the three-panel bench
/// measured. A shirt of five pieces goes round three rings; the heading is
/// written on one tract of one of them, which is one press of "Dar rumbo"; and
/// the other two have to follow the garment they are sewn to. Read before the
/// rule existed, the collar and the band each came out at exactly 180.000°
/// from the body — the middle of their own single panel, which is where a
/// surface with nothing declared opens — the mean seam stood 279.0 mm apart
/// against the 127.2 mm of the same garment declared three times, and the
/// band's centre-front corner stood 345.0 mm from the hem corner it is sewn
/// to instead of 15.0 mm. Nothing on the bar said so: no piece was adrift, the
/// stations all agreed with the body, and the collar was at its own height.
#[test]
#[ignore = "release-only: a real body baked, for the rings a station names"]
fn one_declared_heading_turns_every_ring_of_a_five_piece_shirt() {
    let body = Collider::bake(&reference()).expect("the Anny body is closed and orientable");
    let told = let_go("rumbo en el cuerpo", CUT, Faced::TheBody, &body);
    let thrice = let_go("rumbo en los tres", CUT, Faced::Every, &body);

    assert_eq!(told.stands.len(), 3, "the shirt goes round three rings");
    for (named, turn) in FRONTS.iter().zip(&told.turns) {
        assert!(
            apart(*turn) < SLACK,
            "the {} came out {turn:.3}\u{b0} from the centre front",
            named.1
        );
    }
    // Against the same garment declared three times over, which is the control
    // and the whole of what the rule has to reach: one press has to be worth
    // the three presses nobody should have to make.
    assert!(
        told.seams.1 < thrice.seams.1 * 1.01,
        "one declaration left the seams {:.1} mm apart on the mean against the \
         {:.1} mm of three",
        told.seams.1 * 1000.0,
        thrice.seams.1 * 1000.0
    );
}

/// And with nothing declared the two rings that have no heading of their own
/// really do come out half a turn round, so the scene above is not a reading
/// of a garment that was never in doubt.
#[test]
#[ignore = "release-only: a real body baked, for the rings a station names"]
fn with_nothing_declared_the_collar_and_the_band_face_the_other_way() {
    let body = Collider::bake(&reference()).expect("the Anny body is closed and orientable");
    let adrift = let_go("sin declarar rumbo", CUT, Faced::Nothing, &body);
    for (named, turn) in FRONTS.iter().skip(1).zip(adrift.turns.iter().skip(1)) {
        assert!(
            apart(*turn) > 90.0,
            "the {} came out only {turn:.3}\u{b0} round",
            named.1
        );
    }
}
