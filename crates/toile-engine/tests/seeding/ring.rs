use std::f64::consts::TAU;

use toile_engine::body::Collider;
use toile_engine::couture::ShapePipeline;
use toile_engine::draft::{
    Command, Doc, EdgeRange, Hang, Identity, MeasureSet, Piece, Point, PointKey, Winding, block,
};
use toile_engine::session::{Loose, Session};

use crate::blouse::{self, Cut as Bodice};
use crate::fit::meshed;
use crate::leg::{self, Ease};
use crate::skirt::{self, Cut as Tube};
use crate::watch::reference;

/// How wide the panel that must light the reading is drawn, in centimetres.
///
/// Forty, which is a sheet of cloth and not a strange one: half a metre of a
/// person's circumference is a sleeve, a pocket bag or a yoke. What makes it
/// the wrong thing to hang from a waist is that a waist is two and a half
/// times round it.
const PANEL: f64 = 40.0;

/// And how long, which changes nothing the reading sees.
const LONG: f64 = 50.0;

/// How much bigger than the cloth on it each of a release's rings came off.
///
/// Read against the meshes a test builds, which `meshed` holds to the ones the
/// session is standing on: the cloth a ring carries across is the sum of its
/// own pieces' abscissa extents, which is the very sum the placement matched
/// that ring from.
fn opened(session: &Session) -> Vec<f64> {
    let pipes = meshed(session);
    session
        .rings()
        .iter()
        .map(|ring| {
            let cloth: f64 = ring
                .wraps
                .iter()
                .enumerate()
                .filter(|(_, wrap)| wrap.is_some())
                .map(|(at, _)| across(&pipes[at]))
                .sum();
            ring.radius * TAU / cloth
        })
        .collect()
}

/// How far across one piece's cloth reaches, in metres.
fn across(pipe: &ShapePipeline) -> f64 {
    let (lo, hi) = pipe
        .pos2d
        .iter()
        .fold((f64::MAX, f64::MIN), |(l, h), p| (l.min(p[0]), h.max(p[0])));
    hi - lo
}

/// Lets one document go and prints what its rings read.
fn let_go(scene: &str, doc: Doc, body: &Collider) -> (Vec<f64>, Option<f64>) {
    let session = Session::from_doc(doc, body.clone()).expect("it drapes");
    let read = opened(&session);
    let said = session.loose_ring().map(Loose::opened);
    println!(
        "{scene}: {} anillo(s) \u{b7} {} \u{b7} la barra dice {:?}",
        read.len(),
        read.iter()
            .map(|at| format!("{at:.4}\u{d7}"))
            .collect::<Vec<_>>()
            .join(" | "),
        said.map(|at| format!("{at:.4}\u{d7}"))
    );
    (read, said)
}

/// A lone panel forty centimetres wide, hung from the waist.
///
/// The mistake the reading exists for, in the smallest document that makes it:
/// one piece, one hang, and a station a person typed. Nothing is wrong with
/// the panel and nothing is wrong with the placement — the opening walk lets
/// the surface out until this body's waist is clear, which is its work. What
/// is wrong is the station.
fn panel_from_the_waist() -> Doc {
    let mut doc = Doc::new(MeasureSet::new("Maniquí", []));
    let nodes = [
        ("alto_izq", 0.0, 0.0),
        ("alto_der", PANEL, 0.0),
        ("bajo_der", PANEL, LONG),
        ("bajo_izq", 0.0, LONG),
    ];
    let points: Vec<PointKey> = nodes
        .iter()
        .map(|&(label, x, y)| doc.points.insert(Point::at(x, y).named(label)))
        .collect();
    let piece = doc
        .pieces
        .insert(Piece::polygon("Panel", points, Winding::Cw));
    let named = |label: &str| {
        doc.shows_label(piece, label)
            .expect("the fixture has just named it")
    };
    let at = EdgeRange::between(piece, named("alto_izq"), named("alto_der"));
    Command::AddHang {
        identity: Identity::New,
        hang: Hang::new(at, Hang::WAIST),
    }
    .apply(&mut doc)
    .expect("the panel's top edge runs between two of its own nodes");
    doc
}

/// Every garment this suite places, in the order it reads them.
fn every() -> Vec<(String, Doc)> {
    let mut all: Vec<(String, Doc)> = vec![
        ("el bloque que se envía".to_owned(), block::trousers()),
        ("la pierna igualada".to_owned(), leg::leg(Ease::MATCHED)),
        ("la pierna del dueño".to_owned(), leg::leg(Ease::HIS_JEANS)),
        ("la falda de referencia".to_owned(), skirt::skirt(None)),
        (
            "la falda de cadera ancha".to_owned(),
            skirt::cut_to(Tube::WIDE_HIPPED, None),
        ),
    ];
    let cuts = [
        ("de referencia", Bodice::REFERENCE),
        ("angosta", Bodice::NARROW),
    ];
    for (named, cut) in cuts {
        for (said, station) in [("declarada", Some(blouse::CHEST)), ("sin declarar", None)] {
            all.push((
                format!("la blusa {named}, {said}"),
                blouse::blouse(cut, station),
            ));
        }
    }
    all
}

/// Every garment this suite places comes off a hoop near the size of its own
/// cloth, and the one panel hung from a ring it does not belong on does not.
///
/// The threshold decided by measurement rather than by taste. The widest
/// reading over every scene here is the narrow blouse declared at the chest,
/// 1.161× — a blouse cut for a chest sixteen centimetres smaller than the
/// person wearing it, which is the hardest case the suite carries — and every
/// garment drafted to its own body reads inside a hundredth of 1.0. The lone
/// panel reads 2.09×: it covers 48 % of its ring, its two ends half the ring
/// apart. Nothing sits between those, so a third of the ring bare is the line
/// the engine draws, and the box is quiet on every garment above it.
#[test]
#[ignore = "release-only: a real body baked, for the rings a station names"]
fn every_garment_the_suite_places_comes_off_a_hoop_near_its_own_cloth() {
    let body = Collider::bake(&reference()).expect("the Anny body is closed and orientable");
    let mut worst = (0.0f64, String::new());
    for (named, doc) in every() {
        let (read, said) = let_go(&named, doc, &body);
        assert_eq!(said, None, "{named}: the box lit up at {said:?}");
        for at in read {
            if at > worst.0 {
                worst = (at, named.clone());
            }
        }
    }
    println!(
        "the widest of them all: {:.4}\u{d7} in {}",
        worst.0, worst.1
    );
    assert!(
        worst.0 < 1.25,
        "the widest garment the suite places reads {:.4}\u{d7}, in {}",
        worst.0,
        worst.1
    );

    // And the panel, which is the other end of the same reading: the box says
    // so, and says it with the station a person typed.
    let (read, said) = let_go("el panel de 40 cm", panel_from_the_waist(), &body);
    let said = said.expect("the panel's ring is much bigger than its cloth");
    assert!(
        said > 2.0 && read.iter().all(|&at| at > 2.0),
        "the panel came off {said:.4}\u{d7} its own cloth, rings {read:?}"
    );
}
