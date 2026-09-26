use toile_engine::body::{Collider, Phenotype, body_mesh};
use toile_engine::couture::HOLDS_ITS_RATIO;
use toile_engine::draft::{Command, EdgeAnchor, EdgeRange, Elastic, Hang, Identity};
use toile_engine::session::Session;

use crate::band::{drafted, node, width, written};

/// The band draping, with an elastic along its whole waistline and a hang from
/// the waist over that same stretch.
///
/// The elastic is at its neutral ratio, so a rest length the solver is handed
/// is the cloth's own and the sum of them is a length a tape could check.
fn worn(folded: bool, body: Collider) -> Session {
    let (mut doc, piece) = written(folded);
    let at = EdgeRange::between(piece, node(&doc, piece, "wb_1"), node(&doc, piece, "wb_2"));
    Command::AddElastic {
        identity: Identity::New,
        elastic: Elastic::new(at, Elastic::NEUTRAL_RATIO, HOLDS_ITS_RATIO),
    }
    .apply(&mut doc)
    .expect("both ends are nodes of the band");
    Command::AddHang {
        identity: Identity::New,
        hang: Hang::new(at, Hang::WAIST),
    }
    .apply(&mut doc)
    .expect("both ends are nodes of the band");
    Session::from_doc(doc, body).expect("the band drapes")
}

/// What an elastic over part of the tract leaving `wb_1` comes to hold, from
/// `head` of it to `tail`.
fn over(folded: bool, head: f64, tail: f64) -> Vec<((u32, u32), f32)> {
    let (mut doc, piece) = written(folded);
    let from = node(&doc, piece, "wb_1");
    let along = |t| EdgeAnchor { piece, from, t };
    let at = EdgeRange {
        head: along(head),
        tail: along(tail),
    };
    Command::AddElastic {
        identity: Identity::New,
        elastic: Elastic::new(at, Elastic::NEUTRAL_RATIO, HOLDS_ITS_RATIO),
    }
    .apply(&mut doc)
    .expect("both ends are places on a tract of the band");
    Session::from_doc(doc, Collider::demo())
        .expect("the band drapes")
        .held_cloth()
}

/// How much cloth the product's elastics hold, in centimetres.
fn held_cm(session: &Session) -> f64 {
    let sum: f32 = session.held_cloth().iter().map(|&(_, rest)| rest).sum();
    f64::from(sum) * 100.0
}

/// The mirror of a stretch of the drawn walk is that stretch reflected: as
/// long as it, and opening where its own far end lands.
///
/// Both ends of the crease are their own mirror and a reflection keeps every
/// length, so the drawn walk is half of the cloth's boundary — bit for bit on
/// this piece, which is what the first assertion is here to notice if it ever
/// stops being true. Nothing leans on it: the mirror is taken against the
/// perimeter the cloth measured, not against an assumed half.
#[test]
fn the_mirror_of_a_stretch_of_the_walk_is_that_stretch_reflected() {
    let (folded, piece) = drafted(true);
    let cloth = folded.cloth(piece).expect("the band is drawn to a fold");
    assert_eq!(cloth.perimeter, 2.0 * cloth.walk);
    // The waistline: 47.5 centimetres of the walk, opening 51.5 past where the
    // cloth itself opens.
    let hem = (51.5 / cloth.perimeter, 47.5 / cloth.perimeter);
    let mirror = cloth.mirror_run(hem).expect("the hem is on the drawn walk");
    assert!((mirror.0 - 0.5).abs() < 1.0e-12, "{mirror:?}");
    assert_eq!(mirror.1, hem.1, "as long as the stretch it reflects");
    // A stretch measured the other way round the cloth covers the mirrored
    // half itself, so reflecting it would name the drawn half a second time.
    assert_eq!(cloth.mirror_run((hem.0 + hem.1, 1.0 - hem.1)), None);
}

/// An elastic written on a piece drawn to a fold holds the cloth the piece is
/// cut from, and not the half that was drawn.
///
/// This band's waistline is 47.5 centimetres of drawing and 95 of cloth. Read
/// the drawing alone and somebody who drew one waistband is handed half of one
/// held, with nothing anywhere saying which half.
#[test]
fn an_elastic_at_the_fold_holds_the_whole_cloth_and_not_the_drawn_half() {
    let (folded, piece) = drafted(true);
    let cloth = width(folded.cloth_cm(piece));
    let whole = held_cm(&worn(true, Collider::demo()));
    // Within a boundary sample at each far end: a stretch holds the vertices
    // the mesher laid, and the last one before a corner sits short of it.
    assert!(
        whole <= cloth && whole > cloth - 0.8,
        "{whole} of {cloth} cm"
    );
    let half = held_cm(&worn(false, Collider::demo()));
    assert!(
        whole > 1.9 * half,
        "{whole} cm against the drawing's {half}"
    );
}

/// And a piece nobody folded is held over its one drawn waistline, which is
/// what keeps every scene that carries an elastic where it was.
#[test]
fn a_piece_with_no_fold_is_held_over_the_one_stretch_it_always_was() {
    let (drawn, piece) = drafted(false);
    let drawn_cm = width(drawn.cloth_cm(piece));
    let held = held_cm(&worn(false, Collider::demo()));
    assert!(
        held <= drawn_cm && held > drawn_cm - 0.6,
        "{held} of {drawn_cm} cm"
    );
}

/// A stretch only one half of the cloth has the vertices to carry is not
/// carried at all.
///
/// The mesh lays its own boundary, so six millimetres of hem two thirds of the
/// way along this one come to two vertices on the mirrored half and one on the
/// drawn half. Hold the half that came out and the band sits on the far side of
/// the crease, where nobody drew it: the very defect reading the whole cloth is
/// for, mirrored. Three millimetres more and both halves have the vertices for
/// it, and then all of it is held.
#[test]
fn a_stretch_only_one_half_of_the_cloth_can_carry_is_not_held() {
    assert!(
        over(true, 0.64, 0.652).is_empty(),
        "one half of the cloth is no half to hold the band by"
    );
    assert!(
        !over(true, 0.64, 0.664).is_empty(),
        "a stretch both halves can carry is held"
    );
}

/// A hang written on a piece drawn to a fold holds the cloth too, for the
/// reason the elastic does — counted in vertices of the combined state.
///
/// Each of them once. The drawn stretch and its mirror meet on the crease, so
/// the vertex there is named by both, and a vertex held twice is pulled twice
/// in a substep: twice the step the body's own field leaves room for.
#[test]
#[ignore = "a real body baked, for the rings a station names"]
fn a_hang_at_the_fold_holds_the_whole_cloth_and_holds_each_vertex_once() {
    let mesh = body_mesh(&Phenotype::default(), &[0.0; 20]);
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    let whole = worn(true, body.clone()).hung_cloth();
    let half = worn(false, body).hung_cloth();
    assert!(!half.is_empty(), "the drawn half hangs from the waist");
    assert!(
        whole.len() * 10 > half.len() * 19,
        "{} vertices against the drawing's {}",
        whole.len(),
        half.len()
    );
    let mut once: Vec<u32> = whole.iter().map(|&(v, _)| v).collect();
    let named = once.len();
    once.sort_unstable();
    once.dedup();
    assert_eq!(once.len(), named, "a vertex is held by one pull, not two");
}
