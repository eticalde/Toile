use toile_engine::body::Collider;
use toile_engine::session::Session;

use super::super::super::fit::placed;
use super::super::super::watch::{reference, span};
use super::super::Cut;
use super::{declared, with_a_bag};

/// What one release of the blouse-and-bag came to.
struct Read {
    /// Each ring's height, in metres.
    stands: Vec<f32>,
    /// How many sewn pieces no ring could place.
    adrift: usize,
    /// Lowest and highest released particle, in metres.
    span: (f32, f32),
}

/// Lets one document go over `body` and reads what it placed.
fn let_go(scene: &str, doc: toile_engine::draft::Doc, body: &Collider) -> Read {
    let session = Session::from_doc(doc, body.clone()).expect("it drapes");
    assert!(
        session.seam_faults().is_empty(),
        "{scene}: every seam pairs: {:?}",
        session.seam_faults()
    );
    let read = Read {
        stands: session.rings().iter().map(|ring| ring.stand).collect(),
        adrift: session.adrift(),
        span: span(&placed(&session.released())),
    };
    println!(
        "{scene}: {} anillo(s) a {:?} \u{b7} {} pieza(s) sin sitio \u{b7} tela {:.4}..{:.4}",
        read.stands.len(),
        read.stands,
        read.adrift,
        read.span.0,
        read.span.1
    );
    read
}

/// A component with no station of its own and no seam reaching one is let go
/// flat, and counted, instead of being wrapped round whatever ring its girth
/// happens to match.
///
/// Measured before the rule: the blouse went to its chest ring and the bag —
/// 28 cm of cloth sewn to nothing else — was placed on a 0.27042 m hoop at
/// −0.6927 m, which on this body is an ankle, with its cloth reaching
/// −0.8727 m against a sole at −0.8371. `adrift()` read 0, so the bar said
/// nothing at all. The bag has no reason to be anywhere: flat over the person
/// is what the tree has always done with cloth it cannot place, and the count
/// is what reaches a person.
#[test]
#[ignore = "release-only: a real body baked, for the rings a station names"]
fn a_component_no_declaration_reaches_is_let_go_flat_and_counted() {
    let body = Collider::bake(&reference()).expect("the Anny body is closed and orientable");
    let chest = body
        .belt_at(super::super::CHEST)
        .copied()
        .expect("the baked body carries its chest ring");
    let told = let_go(
        "blusa declarada, bolsa suelta",
        declared(Cut::REFERENCE),
        &body,
    );

    assert_eq!(told.stands, [chest.height], "the blouse alone is placed");
    assert_eq!(told.adrift, 2, "and the bag's two panels are counted");
    let (lo, _) = body.extent();
    assert!(
        told.span.0 > lo[1],
        "no cloth went under the body's own soles: {:.4} against {:.4}",
        told.span.0,
        lo[1]
    );
}

/// And a document that declares nothing at all keeps the release it always
/// had: nothing is placed, and the count says how much.
///
/// The control, and the half that says the rule above is about the
/// declaration and not about the bag. Two components with nothing declared
/// anywhere is the shape the one strip walk refused outright before any of
/// this existed — its guard was that the walk had to place every sewn piece of
/// the product — so both of these go flat, exactly as they did, and the five
/// pieces are counted.
#[test]
#[ignore = "release-only: a real body baked, for the rings a station names"]
fn two_components_with_nothing_declared_are_the_release_the_tree_always_ran() {
    let body = Collider::bake(&reference()).expect("the Anny body is closed and orientable");
    let guessed = let_go("sin declarar nada", with_a_bag(Cut::REFERENCE, None), &body);
    assert!(guessed.stands.is_empty(), "nothing is placed at all");
    assert_eq!(guessed.adrift, 5, "and all five sewn pieces are counted");
}

/// A product of one component and no declaration is placed, which is the
/// reduction every drape golden and the owner's jeans stand on.
///
/// Read here beside the two above, because what tells them apart is the one
/// thing the rule turns on: a group nobody declared is placed where it is the
/// whole product, and not where it is one component of several.
#[test]
#[ignore = "release-only: a real body baked, for the rings a station names"]
fn one_component_and_no_declaration_is_still_placed() {
    let body = Collider::bake(&reference()).expect("the Anny body is closed and orientable");
    let alone = let_go(
        "un solo grupo, sin declarar",
        super::super::blouse(Cut::REFERENCE, None),
        &body,
    );
    assert_eq!(alone.stands.len(), 1, "the one component is placed");
    assert_eq!(alone.adrift, 0, "and nothing is counted");
}
