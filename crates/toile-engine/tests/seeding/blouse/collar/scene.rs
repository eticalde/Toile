use toile_engine::body::Collider;
use toile_engine::session::Session;

use super::super::super::fit::{placed, report};
use super::super::super::watch::{reference, span};
use super::super::{CHEST, Cut};
use super::{NECK, collared, facing_the_front};

/// What one release of the collared blouse came to.
struct Read {
    /// The rings it was let go on: the height each stands at and how far round
    /// the hoop its hung line landed on goes, in metres.
    rings: Vec<(f32, f64)>,
    /// How many sewn pieces no ring could place.
    adrift: usize,
    /// Lowest and highest released particle, in metres.
    span: (f32, f32),
    /// Worst and mean seam gap at release, in metres.
    seams: Option<(f32, f32)>,
}

/// Lets the collared blouse go over `body` and reads where it went.
///
/// The release and not a drape, for the reason the station scenes measure the
/// release: what a declaration changes is where the cloth is put down, and that
/// is decided before the solver has run a substep.
fn let_go(scene: &str, doc: toile_engine::draft::Doc, body: &Collider) -> Read {
    let session = Session::from_doc(doc, body.clone()).expect("the blouse drapes");
    assert!(
        session.seam_faults().is_empty(),
        "{scene}: every seam of the collared blouse pairs: {:?}",
        session.seam_faults()
    );
    let points = placed(&session.released());
    let read = Read {
        rings: session
            .rings()
            .iter()
            .map(|ring| (ring.stand, ring.radius * std::f64::consts::TAU))
            .collect(),
        adrift: session.adrift(),
        span: span(&points),
        seams: report(scene, "at release", &session.sewn_pairs(), &points),
    };
    let rings: Vec<String> = read
        .rings
        .iter()
        .map(|&(stand, hoop)| format!("{stand:.4} m \u{b7} aro {hoop:.5} m"))
        .collect();
    println!(
        "{scene}: {} anillo(s) [{}] \u{b7} {} pieza(s) sin sitio \u{b7} tela {:.4}..{:.4}",
        read.rings.len(),
        rings.join(" | "),
        read.adrift,
        read.span.0,
        read.span.1
    );
    read
}

/// The collar strip takes the whole garment off the body, and declaring its own
/// ring puts the garment back on.
///
/// The figure of this part. The strip is sewn across all three chest lines, so
/// the back panel carries three seams and a chain has room for two: with the
/// strip declaring nothing the walk gives up, nothing is placed, and all four
/// pieces go to the flat release over the crown. Declaring the neck ring splits
/// the garment into the two rings it really goes round — the body on the chest,
/// the strip on the neck — and every piece lands on the person.
#[test]
#[ignore = "release-only: a real body baked, for the rings a station names"]
fn a_collar_strip_takes_the_garment_off_the_body_until_it_says_where_it_goes() {
    let body = Collider::bake(&reference()).expect("the Anny body is closed and orientable");
    let (lo, hi) = body.extent();
    let chest = *body
        .belt_at(CHEST)
        .expect("the body carries its chest ring");
    let neck = *body.belt_at(NECK).expect("and its neck ring");
    println!(
        "the body: skin {:.4}..{:.4} \u{b7} {CHEST} {:.4} m at {:.4} \u{b7} {NECK} {:.4} m at {:.4}",
        lo[1], hi[1], chest.girth, chest.height, neck.girth, neck.height
    );

    let cut = Cut::REFERENCE;
    let adrift = let_go("sin declarar la tira", collared(cut, None, None), &body);
    let told = let_go("la tira declara el anillo", facing_the_front(cut), &body);

    // Before: not a near miss but the whole garment off the person, its top
    // above the crown, and the four pieces counted rather than lost.
    assert!(adrift.rings.is_empty(), "nothing is placed at all");
    assert_eq!(adrift.adrift, 4, "and all four pieces are counted");
    assert!(
        adrift.span.1 > neck.height,
        "the cloth reaches {:.1} mm over the {NECK} ring",
        f64::from(adrift.span.1 - neck.height) * 1000.0
    );

    // After: two rings, each standing on the ring its own cloth was hung from,
    // and the heights are the body's own copied rather than worked out here.
    assert_eq!(told.adrift, 0, "every piece has a place");
    let stands: Vec<f32> = told.rings.iter().map(|&(stand, _)| stand).collect();
    assert_eq!(
        stands,
        [chest.height, neck.height],
        "the body of the blouse on the chest, the strip on the neck"
    );

    // And the garment is on the person: no cloth over the crown, which is the
    // whole of what the defect was.
    assert!(
        told.span.1 <= hi[1],
        "the cloth tops out at {:.4} against the crown's {:.4}",
        told.span.1,
        hi[1]
    );
    // What the second ring above the chest costs, measured rather than hoped
    // for. The strip's own surface is a surface of revolution spanning the
    // heights from the chest to the neck, and the arms stand out sideways there
    // — `brazo_contorno` is 0.3627 m round centred 0.2325 m off the axis, so
    // its outer skin reaches 0.29 m out — so the clearance walk opens that
    // surface until it is past them. The strip is then let go a hand's
    // breadth wide of the chest line it is sewn to, and the seam between
    // the two rings carries the difference.
    let arms = f64::from(ARM_REACH) * std::f64::consts::TAU;
    println!(
        "the strip's hoop: {:.5} m at its own neck line, {:.5} m at the chest line \u{b7} \
         clearing the arms wants {arms:.5} m",
        told.rings[1].1,
        hoop_at(&body, cut, 0.0)
    );
    assert!(
        hoop_at(&body, cut, 0.0) > arms,
        "the strip is opened to {:.5} m at the chest line, past the arms",
        hoop_at(&body, cut, 0.0)
    );

    // So the seams do not shut at release here, and the reading says which of
    // the two facts that is: the mean gap falls by more than half, because four
    // of the five seams are the body of the blouse closing as it always did,
    // while the worst stays the width the arms asked for.
    let (shut, open) = (
        told.seams.expect("the collared blouse carries five seams"),
        adrift.seams.expect("and so does the control"),
    );
    println!(
        "worst seam {:.1} mm placed against {:.1} mm flat \u{b7} mean {:.1} mm against {:.1} mm",
        shut.0 * 1000.0,
        open.0 * 1000.0,
        shut.1 * 1000.0,
        open.1 * 1000.0
    );
    assert!(
        f64::from(shut.1) < f64::from(open.1) / 2.0,
        "the mean seam gap is {:.1} mm placed and {:.1} mm flat",
        shut.1 * 1000.0,
        open.1 * 1000.0
    );
}

/// How far the reference body's arms reach from its own axis, in metres.
///
/// `brazo_contorno` measures 0.3627 m round with its middle 0.2325 m off the
/// axis, so its skin reaches that far plus its own radius. Read as a constant
/// because it is a reading of the body and not of the garment: a surface of
/// revolution round this person at chest height has to be at least this wide or
/// it is inside an arm.
const ARM_REACH: f32 = 0.2325 + 0.0577;

/// How far round the collar strip's own surface goes at one pattern ordinate of
/// it, in metres.
fn hoop_at(body: &Collider, cut: Cut, y: f64) -> f64 {
    let session = Session::from_doc(facing_the_front(cut), body.clone()).expect("it drapes");
    let rings = session.rings();
    let strip = rings.get(1).expect("the strip is the second ring");
    strip.round.radius(y) * std::f64::consts::TAU
}
