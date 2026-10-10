use toile_engine::body::Collider;
use toile_engine::session::Session;

use super::super::super::fit::{placed, report};
use super::super::super::watch::{reference, span};
use super::super::{CHEST, blouse};
use super::{HIP, TO_THE_HIP, banded, facing_the_front};

/// What one release of the banded blouse came to.
struct Read {
    /// The height each ring stands at, and how far round the hoop its hung line
    /// landed on goes, in metres.
    rings: Vec<(f32, f64)>,
    /// How many sewn pieces no ring could place.
    adrift: usize,
    /// Lowest and highest released particle, in metres.
    span: (f32, f32),
    /// Worst and mean seam gap at release, in metres.
    seams: Option<(f32, f32)>,
}

/// Lets a banded blouse go over `body` and reads where it went.
fn let_go(scene: &str, doc: toile_engine::draft::Doc, body: &Collider) -> Read {
    let session = Session::from_doc(doc, body.clone()).expect("the blouse drapes");
    assert!(
        session.seam_faults().is_empty(),
        "{scene}: every seam pairs: {:?}",
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

/// A band sewn across three hems takes the garment off the body until it says
/// which ring it goes on, and then the whole garment is let go on the person.
///
/// The clean half of the figure. The band gives itself three seams, so before a
/// second ring could be placed the chain gave up and all four pieces went to
/// the flat release over the crown. The band's ring is below the chest, where
/// nothing of the body stands out sideways, so this scene reads what the
/// placement does with nothing in its way: both rings land on the ring they
/// name, and the seams come down to what a drape shuts.
#[test]
#[ignore = "release-only: a real body baked, for the rings a station names"]
fn a_band_across_three_hems_is_placed_once_it_says_which_ring_it_is_on() {
    let body = Collider::bake(&reference()).expect("the Anny body is closed and orientable");
    let (_, hi) = body.extent();
    let chest = *body
        .belt_at(CHEST)
        .expect("the body carries its chest ring");
    let hip = *body.belt_at(HIP).expect("and its hip ring");
    println!(
        "the body: {CHEST} {:.4} m at {:.4} \u{b7} {HIP} {:.4} m at {:.4}",
        chest.girth, chest.height, hip.girth, hip.height
    );

    let cut = TO_THE_HIP;
    let adrift = let_go("sin declarar la pretina", banded(cut, None, None), &body);
    let told = let_go("la pretina declara el anillo", facing_the_front(cut), &body);

    assert!(adrift.rings.is_empty(), "nothing is placed at all");
    assert_eq!(adrift.adrift, 4, "and all four pieces are counted");
    assert_eq!(told.adrift, 0, "declared, every piece has a place");
    let stands: Vec<f32> = told.rings.iter().map(|&(stand, _)| stand).collect();
    assert_eq!(
        stands,
        [chest.height, hip.height],
        "the body of the blouse on the chest, the band on the hip"
    );
    assert!(
        told.span.1 <= hi[1],
        "the cloth tops out at {:.4} against the crown's {:.4}",
        told.span.1,
        hi[1]
    );

    let (shut, open) = (
        told.seams.expect("the banded blouse carries five seams").0,
        adrift.seams.expect("and so does the control").0,
    );
    println!(
        "worst seam: {:.1} mm placed against {:.1} mm flat",
        shut * 1000.0,
        open * 1000.0
    );
    // An order of magnitude, not a margin: the control's pieces are stacked on
    // each other in one plane and its seams stand most of a panel apart.
    assert!(
        f64::from(shut) < f64::from(open) / 10.0,
        "the worst seam starts {:.1} mm open placed and {:.1} mm flat",
        shut * 1000.0,
        open * 1000.0
    );
}

/// The band's own ring moves no vertex of the body of the blouse.
///
/// The owner's first limit, asked of the smallest case that can answer it: the
/// same three panels, hung from the same station, with and without a second
/// ring beside them. Every vertex of every panel is placed to the bit either
/// way, so a garment that gains a ring does not have the ring it already had
/// re-read.
#[test]
#[ignore = "release-only: a real body baked, for the rings a station names"]
fn a_second_ring_moves_no_vertex_of_the_first() {
    let body = Collider::bake(&reference()).expect("the Anny body is closed and orientable");
    let cut = TO_THE_HIP;
    let alone = Session::from_doc(blouse(cut, Some(CHEST)), body.clone()).expect("it drapes");
    let with = Session::from_doc(banded(cut, Some(HIP), None), body).expect("it drapes too");
    let (one, two) = (
        alone.layout().expect("the three panels chain"),
        with.layout().expect("and they still do"),
    );
    assert_eq!(two.stand.to_bits(), one.stand.to_bits());
    assert_eq!(two.crest.to_bits(), one.crest.to_bits());
    assert_eq!(two.radius.to_bits(), one.radius.to_bits());

    // Every vertex of the three panels, through the one call every released
    // vertex goes through.
    let (flat, banded) = (placed(&alone.released()), placed(&with.released()));
    let panels: usize = alone
        .pieces()
        .iter()
        .filter_map(|&piece| alone.offset(piece))
        .count();
    assert_eq!(panels, 3, "the plain blouse is the three panels");
    let moved = flat
        .iter()
        .zip(&banded)
        .filter(|(a, b)| a.map(f32::to_bits) != b.map(f32::to_bits))
        .count();
    assert_eq!(
        moved,
        0,
        "{moved} of the plain blouse's {} vertices moved",
        flat.len()
    );
}
