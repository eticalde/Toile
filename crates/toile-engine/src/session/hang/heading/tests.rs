use toile_doc::{Command, Doc, EdgeRange, Hang, Identity, MeasureSet, Piece, Point, Winding};

use super::*;
use crate::body::Collider;
use crate::draft::PieceKey;

/// A panel two metres of pattern wide, with its top edge named left to
/// right and its hem named under it.
///
/// Wide and shallow on purpose: every reading here is of an abscissa, and a
/// panel whose corners are far apart in x leaves no doubt which end of a
/// run the pin landed on.
fn panel() -> (Doc, PieceKey) {
    drawn(&[("a", 0.0, 0.0), ("b", 200.0, 0.0), ("c", 200.0, 40.0)])
}

/// The same panel with a notch cut into its waistline: the smallest concave
/// piece, and the shape a placket opening, a zip or a V neck takes.
///
/// The notch is drawn well to the left of the middle on purpose. Its left wall
/// has the cloth to its left, 40 cm of pattern of it, while the panel's own
/// right-hand canto stands 160 cm away on the other side — so a reading that
/// compared how far the cloth reached either way would answer the wrong one.
fn notched() -> (Doc, PieceKey) {
    let corners = [
        ("a", 0.0, 0.0),
        ("muesca_arriba_izq", 40.0, 0.0),
        ("muesca_pie_izq", 40.0, 20.0),
        ("muesca_pie_der", 80.0, 20.0),
        ("muesca_arriba_der", 80.0, 0.0),
        ("b", 200.0, 0.0),
        ("c", 200.0, 40.0),
        ("d", 0.0, 40.0),
    ];
    drawn(&corners)
}

/// The same panel with a vent in it: four millimetres of opening, twelve
/// centimetres deep, with a node partway down each of its two walls.
///
/// Narrower than the mesh's own step down a run that crosses it, which is what
/// makes it the one shape [`beside`] has no answer for.
fn vented() -> (Doc, PieceKey) {
    let corners = [
        ("a", 0.0, 0.0),
        ("abertura_arriba_izq", 40.0, 0.0),
        ("abertura_alta", 40.0, 1.0),
        ("abertura_pie_izq", 40.0, 12.0),
        ("abertura_pie_der", 40.4, 12.0),
        ("abertura_baja", 40.4, 11.0),
        ("abertura_arriba_der", 40.4, 0.0),
        ("b", 200.0, 0.0),
        ("c", 200.0, 40.0),
        ("d", 0.0, 40.0),
    ];
    drawn(&corners)
}

/// One piece drawn straight through the corners it is given, each named.
fn drawn(corners: &[(&str, f64, f64)]) -> (Doc, PieceKey) {
    let mut doc = Doc::new(MeasureSet::new("Maniquí", []));
    let points = corners
        .iter()
        .map(|&(label, x, y)| doc.points.insert(Point::at(x, y).named(label)))
        .collect::<Vec<_>>();
    let piece = doc
        .pieces
        .insert(Piece::polygon("Panel", points, Winding::Cw));
    (doc, piece)
}

/// The panel hung from the waist by the run between two of its nodes,
/// facing `heading`.
fn hung(from: &str, to: &str, heading: Option<Heading>) -> Session {
    let (doc, piece) = panel();
    hung_on(doc, piece, from, to, heading)
}

/// The same, over whichever piece of whichever document is handed in.
fn hung_on(
    mut doc: Doc,
    piece: PieceKey,
    from: &str,
    to: &str,
    heading: Option<Heading>,
) -> Session {
    let at = |l: &str| doc.shows_label(piece, l).expect("the piece names it");
    let run = EdgeRange::between(piece, at(from), at(to));
    let hang = match heading {
        Some(heading) => Hang::facing(run, Hang::WAIST, heading),
        None => Hang::new(run, Hang::WAIST),
    };
    Command::AddHang {
        identity: Identity::New,
        hang,
    }
    .apply(&mut doc)
    .expect("both ends are nodes of the panel");
    Session::from_doc(doc, Collider::demo()).expect("the panel drapes")
}

/// The one pin a session that declares a single heading reads.
fn one(session: &Session) -> Pin {
    let pins = session.pinned();
    assert_eq!(pins.len(), 1, "one heading, one pin");
    pins[0]
}

/// A hang that says nothing about which way it faces pins nothing, which is
/// what leaves every pattern already drawn placed exactly as it was.
#[test]
fn a_hang_with_no_heading_pins_nothing() {
    assert!(hung("a", "b", None).pinned().is_empty());
}

/// The head of the run is pinned and the tail is not, and `pin` walks from
/// one to the other.
///
/// The two ends of this run are two metres of pattern apart, so this is not
/// a reading that could come out of a rounding: 0 is the drawn `a`, 1 is
/// the drawn `b`, and a half is the middle nobody declared.
#[test]
fn the_pin_walks_from_the_head_of_the_run_to_its_tail() {
    for (pin, at) in [(0.0, 0.0), (0.25, 0.50), (0.5, 1.00), (1.0, 2.00)] {
        let heading = Heading {
            pin,
            ..Heading::facing(0.0, Sense::Leftward)
        };
        let pinned = one(&hung("a", "b", Some(heading)));
        assert!(
            (pinned.at - at).abs() < 0.02,
            "a pin of {pin} landed at {} where {at} was drawn",
            pinned.at
        );
    }
}

/// Which way the run is walked decides what its declared sense means about
/// the piece, because a sense is written about the run and the surface
/// places pieces.
///
/// `a` to `b` runs up the pattern's abscissa and `b` to `a` runs down it,
/// so one declaration out of the two has to come back reversed — and it
/// is the same edge of the same panel either way, which is what makes
/// this a reading of the drawing and not of a convention.
#[test]
fn a_run_walked_the_other_way_reverses_what_its_sense_says_of_the_piece() {
    let leftward =
        |from, to| one(&hung(from, to, Some(Heading::facing(0.0, Sense::Leftward)))).leftward;
    assert_eq!(leftward("a", "b"), 1.0);
    assert_eq!(leftward("b", "a"), -1.0);
    let rightward = one(&hung(
        "a",
        "b",
        Some(Heading::facing(0.0, Sense::Rightward)),
    ))
    .leftward;
    assert_eq!(rightward, -1.0, "and so does declaring the other sense");
}

/// A run straight down the pattern is pinned by the cloth beside it, which
/// is the sentence of the trade: a vertical canto is a centre front, and
/// the garment goes on toward the cloth.
///
/// `b` to `c` is the panel's own right-hand edge and every vertex of the
/// piece stands at or below its abscissa, so the declared sense comes back
/// reversed about the piece — exactly as it does for a run walked from a
/// high abscissa down to a low one.
#[test]
fn a_run_that_crosses_no_cloth_is_pinned_by_the_cloth_beside_it() {
    let heading = Heading::facing(0.0, Sense::Leftward);
    let pinned = one(&hung("b", "c", Some(heading)));
    assert_eq!(pinned.leftward, -1.0);
    assert!(
        (pinned.at - 2.0).abs() < 0.02,
        "the pin is on that edge, at {}",
        pinned.at
    );
}

/// One notched piece carries two vertical cantos with the cloth on opposite
/// sides of them, and each is pinned by the cloth that is beside *it*.
///
/// The reading no comparison of reaches can give, and the one a garment needs:
/// the notch's two walls stand over the same ordinates of the same piece, so
/// whatever answers for one has to answer the other way for the other. Measured
/// before the walk decided it, the left wall came back `1.0` — the panel's
/// right-hand canto is 160 cm of pattern off where its left-hand one is 40 —
/// and a 84.0 cm tube declared on such a wall came out mirrored, its cloth 18
/// cm round from the line reading −76.90° where the contract puts it at
/// +77.37°.
#[test]
fn each_canto_of_a_notched_piece_is_pinned_by_the_cloth_beside_that_canto() {
    let leftward = |from, to| {
        let (doc, piece) = notched();
        let heading = Heading::facing(0.0, Sense::Leftward);
        one(&hung_on(doc, piece, from, to, Some(heading))).leftward
    };
    assert_eq!(leftward("b", "c"), -1.0, "the panel's own right-hand canto");
    assert_eq!(
        leftward("muesca_arriba_izq", "muesca_pie_izq"),
        -1.0,
        "the notch's left wall has the cloth to its left"
    );
    assert_eq!(
        leftward("muesca_pie_der", "muesca_arriba_der"),
        1.0,
        "and its right wall has the cloth to its right"
    );
}

/// And a run down one wall of a vent and up the other pins nothing, because
/// its two walls say opposite things about the same ordinates.
///
/// The one drawing the reading gives up on. A vent narrower than the mesh's own
/// step down the run reads as a canto by its two ends and is not one: there is
/// cloth both sides of it, and either answer would be a guess. A product pinned
/// at an angle is still free to come out mirrored, so the gap is worth more —
/// and it is a gap a person is told about rather than one that happens quietly.
#[test]
fn a_run_down_one_wall_of_a_vent_and_up_the_other_pins_nothing() {
    let (doc, piece) = vented();
    let heading = Heading::facing(0.0, Sense::Leftward);
    let session = hung_on(doc, piece, "abertura_alta", "abertura_baja", Some(heading));
    assert!(
        session.pinned().is_empty(),
        "a vent is not a canto: {:?}",
        session.pinned()
    );
    assert_eq!(session.headless().len(), 1, "and the press is told so");
}

/// The turn reaches the surface in radians, because that is what rolls
/// cloth; the document holds the degrees a person typed.
#[test]
fn the_declared_turn_reaches_the_surface_in_radians() {
    for turn in [0.0, 90.0, 180.0, -90.0] {
        let heading = Heading::facing(turn, Sense::Leftward);
        let pinned = one(&hung("a", "b", Some(heading)));
        assert!(
            (pinned.turn - turn.to_radians()).abs() < 1.0e-12,
            "{turn}° reached the surface as {} rad",
            pinned.turn
        );
    }
}
