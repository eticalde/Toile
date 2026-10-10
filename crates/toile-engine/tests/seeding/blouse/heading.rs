use toile_engine::body::Collider;
use toile_engine::draft::{Heading, Sense};
use toile_engine::session::Session;

use super::super::watch::reference;
use super::{Cut, ORDERS, Scene, across, staged};

/// How far a reading may stand from the place the trade names it, in degrees.
///
/// Mesh and not rule, and the arithmetic is exact. Every reading here is asked
/// at an abscissa the pattern draws, while the hoop it is divided by is the
/// cloth the mesher laid: the top edge of this blouse carries 1.01652 m of the
/// 1.04000 m drawn, 2.31 % short, so a line a quarter of the way round reads
/// 2.06° further round than it was drawn and the back's middle 1.32°. Measured,
/// those two are the worst of the three. Everything this bench claims is tens
/// of degrees from everything it denies.
const SLACK: f64 = 2.5;

/// The three lines this bench reads: the panel, the fraction across it, where
/// the trade says it belongs, and its name.
///
/// The first says whether the declaration was honoured at all. The second says
/// the back went to the back. The third is the one no angle alone settles: a
/// mirrored garment puts the wearer's left side on their right, and reads −90°
/// where the trade says +90°.
const LINES: [(usize, f64, f64, &str); 3] = [
    (0, 0.0, 0.0, "centro delantero"),
    (1, 0.5, 180.0, "medio de la espalda"),
    (0, 1.0, 90.0, "costado izquierdo"),
];

/// Where a blouse's three read lines came out, in degrees from the centre
/// front, and how many of its panels run against the turn.
struct Faced {
    turns: [f64; 3],
    against: usize,
}

impl Faced {
    /// How far the worst of the three readings stands from where the trade
    /// names it, in degrees.
    fn adrift(&self) -> f64 {
        LINES
            .iter()
            .zip(self.turns)
            .map(|(&(_, _, want, _), got)| apart(got, want))
            .fold(0.0, f64::max)
    }
}

/// Lets one scene go on `body` and reads where it faces.
fn let_go(cut: Cut, scene: Scene, body: &Collider) -> Faced {
    let session = Session::from_doc(staged(cut, scene), body.clone()).expect("the blouse drapes");
    assert!(
        session.seam_faults().is_empty(),
        "every seam of the blouse pairs: {:?}",
        session.seam_faults()
    );
    let ring = session.layout().expect("the seams chain the three panels");
    let turns = LINES.map(|(panel, along, _, name)| {
        let at = stored(scene.order, panel);
        ring.round
            .facing(at, across(cut, panel, along), ring.crest)
            .unwrap_or_else(|| panic!("the strip carries {name}"))
    });
    Faced {
        turns,
        against: ring
            .wraps
            .iter()
            .flatten()
            .filter(|wrap| wrap.sense < 0.0)
            .count(),
    }
}

/// Where panel `k` stands in a document whose panels are stored in `order`.
fn stored(order: [usize; 3], k: usize) -> usize {
    order
        .iter()
        .position(|&panel| panel == k)
        .expect("every panel is stored once")
}

/// How far two turns stand apart, the short way round.
fn apart(a: f64, b: f64) -> f64 {
    let gap = (a - b).rem_euclid(360.0);
    gap.min(360.0 - gap)
}

/// Every scene the bench measures: three storage orders, open and buttoned.
fn scenes() -> Vec<(String, Scene)> {
    let mut all = Vec::new();
    for (order, named) in ORDERS {
        for (buttoned, shut) in [(false, "abierta"), (true, "botonada")] {
            all.push((
                format!("{shut}, {named}"),
                Scene {
                    order,
                    buttoned,
                    ..Scene::PLAIN
                },
            ));
        }
    }
    all
}

/// One declared heading fixes the whole garment, whatever order the document
/// holds its panels in — and nothing declared leaves the storage order
/// deciding.
///
/// The figure of the whole part. Six releases of one blouse: three storage
/// orders, each open at the centre front and buttoned. Declared, all six put
/// the left front's centre-front edge at the centre front, the back's middle at
/// the back and the wearer's left side on their left. Undeclared, at least one
/// of them does not, by tens of degrees.
///
/// On the demo body, which carries no rings: a turn is an angle round an axis
/// and no part of it is read off a girth, so the one thing a baked body would
/// add here is minutes of bake for a reading that cannot change.
#[test]
fn one_declared_heading_fixes_the_garment_in_every_storage_order() {
    let body = Collider::demo();
    let cut = Cut::REFERENCE;
    let mut worst = (0.0, String::new());
    for (named, scene) in scenes() {
        let told = let_go(cut, scene.facing_the_front(), &body);
        let guessed = let_go(cut, scene, &body);
        println!(
            "{named}: declarada {:7.3}° {:8.3}° {:8.3}° ({} contra el giro) \u{b7} \
             sin declarar {:7.3}° {:8.3}° {:8.3}° ({} contra)",
            told.turns[0],
            told.turns[1],
            told.turns[2],
            told.against,
            guessed.turns[0],
            guessed.turns[1],
            guessed.turns[2],
            guessed.against
        );
        if told.adrift() > worst.0 {
            worst = (told.adrift(), named.clone());
        }
        assert_eq!(told.against, 0, "{named}: every panel runs with the turn");
    }
    assert!(
        worst.0 < SLACK,
        "the worst declared reading of six was {:.3}° out, in {}",
        worst.0,
        worst.1
    );
}

/// And the storage order really does decide it when nothing is declared, so the
/// scene above is not six readings of a garment that was never in doubt.
#[test]
fn with_nothing_declared_the_storage_order_turns_the_garment() {
    let body = Collider::demo();
    let cut = Cut::REFERENCE;
    let mut worst = (0.0, String::new());
    for (named, scene) in scenes() {
        let adrift = let_go(cut, scene, &body).adrift();
        if adrift > worst.0 {
            worst = (adrift, named);
        }
    }
    assert!(
        worst.0 > 30.0,
        "the worst undeclared reading of six was only {:.3}° out, in {}",
        worst.0,
        worst.1
    );
}

/// The case no angle settles: undeclared, the blouse stored right front first
/// comes out mirrored, and the sense is what rights it.
///
/// Measured, that scene is the one where the garment is not merely turned but
/// inside out — all 3 of its 3 panels run against the turn and the wearer's
/// left side comes out at −136.691°, on their right — so it is the scene that
/// says why a heading carries two words and not one angle.
///
/// Open at the centre front, which is where it was measured and not where it
/// was expected. Buttoned, the same storage order comes out turned 44.627° and
/// not mirrored, because a closed strip is opened at whichever of the first
/// piece's two seams sits further along it and that tie-break happens to run
/// this garment the right way. Declaring the other sense is then the mirror
/// asked for rather than suffered.
#[test]
fn the_blouse_stored_right_front_first_is_righted_by_the_sense() {
    let body = Collider::demo();
    let cut = Cut::REFERENCE;
    let scene = Scene {
        order: [2, 1, 0],
        ..Scene::PLAIN
    };
    let adrift = let_go(cut, scene, &body);
    println!(
        "sin declarar: {:?}, {} contra",
        adrift.turns, adrift.against
    );
    assert_eq!(adrift.against, 3, "3 of 3 panels run against the turn");
    assert!(
        adrift.turns[2] < -90.0,
        "and the wearer's left side is on their right, at {:.3}°",
        adrift.turns[2]
    );

    let told = let_go(cut, scene.facing_the_front(), &body);
    assert!(
        told.adrift() < SLACK,
        "the declaration rights it: {:?}",
        told.turns
    );
    assert_eq!(told.against, 0, "and every panel runs with the turn");

    // The same run declared the other way round is the mirror, asked for: the
    // pin holds the centre front where it was put and the garment goes round
    // the other way from it, which is the whole of what a sense carries.
    let mirrored = Scene {
        heading: Some(Heading::facing(0.0, Sense::Rightward)),
        ..scene
    };
    let other = let_go(cut, mirrored, &body);
    println!("al revés: {:?}, {} contra", other.turns, other.against);
    assert!(
        apart(other.turns[0], 0.0) < SLACK,
        "the centre front is still pinned: {:.3}°",
        other.turns[0]
    );
    assert!(
        apart(other.turns[2], -90.0) < SLACK,
        "and the wearer's left side is on their right: {:.3}°",
        other.turns[2]
    );
    assert_eq!(other.against, 3, "every panel runs against the turn");
}

/// The same six releases over the real body, which is the garment this bench
/// is about.
///
/// A scene of its own rather than a second assertion, because the body changes
/// the arithmetic in one place: an open strip has no whole lap to divide, so
/// its pieces abut at the surface's own radius and the room the body asked for
/// is part of that radius. A closed one divides the lap and the body cannot
/// reach it at all. What does not change either way is the figure — one
/// declaration, one placement, whatever order the file holds.
#[test]
#[ignore = "release-only: a real body baked, for the rings a station names"]
fn one_declared_heading_fixes_the_garment_over_the_body_too() {
    let body = Collider::bake(&reference()).expect("the Anny body is closed and orientable");
    let cut = Cut::REFERENCE;
    let mut told: Vec<[f64; 3]> = Vec::new();
    for (named, scene) in scenes() {
        let faced = let_go(cut, scene.facing_the_front(), &body);
        let guessed = let_go(cut, scene, &body);
        println!(
            "{named}: declarada {:7.3}° {:8.3}° {:8.3}° ({} contra el giro) \u{b7} \
             sin declarar {:7.3}° {:8.3}° {:8.3}° ({} contra)",
            faced.turns[0],
            faced.turns[1],
            faced.turns[2],
            faced.against,
            guessed.turns[0],
            guessed.turns[1],
            guessed.turns[2],
            guessed.against
        );
        assert_eq!(faced.against, 0, "{named}: every panel runs with the turn");
        told.push(faced.turns);
    }
    // Every one of the six read the same three lines to the bit, which is the
    // claim stated as strongly as it can be: not "within a degree" but "the
    // storage order reached no part of this".
    for turns in &told {
        assert_eq!(
            turns.map(f64::to_bits),
            told[0].map(f64::to_bits),
            "{turns:?} against {:?}",
            told[0]
        );
    }
}
