use std::f32::consts::SQRT_2;

use toile_engine::body::{Collider, bake};
use toile_engine::couture::SEAM_SHUT;
use toile_engine::draft::{Doc, Draft, Lengths, Seam, tolerance_cm};
use toile_engine::session::Session;

use crate::fit::{apart, placed, report};
use crate::leg::{Ease, leg};
use crate::watch::{at_mark, buried, reference, touching};

/// How near the solved mismatch has to come to the one the cut asked for, in
/// centimetres.
///
/// A hundredth, which is the precision the report prints at: the fixture solves
/// the back's length in closed form, so anything looser would let a fixture
/// that no longer carries his mismatch go on calling itself his.
const CUT_TO: f64 = 0.01;

/// How far apart the mesher lays the boundary vertices a seam is paired onto,
/// in metres.
///
/// [`SEAM_SHUT`] is a tenth of that spacing by its own definition, so this is
/// the one place the spacing can be named without a second constant to drift
/// from it. It is the floor under any release gap: a side is paired to the
/// nearest boundary vertex, so even an exact map lands the two sides up to one
/// spacing apart in ordinate.
const BOUNDARY: f32 = 10.0 * SEAM_SHUT;

/// How far apart the body itself can hold a sewn pair at the mark, in metres.
///
/// Two cells of the baked field, which is the bound the shipped block's own
/// scene is held to and for its reason: a pair the closing phase cannot shut is
/// pinched between the seam pulling one way and the person pushing the other,
/// and the contact solve works at the field's resolution.
const HELD_OPEN: f32 = 2.0 * bake::CELL as f32;

/// How far apart the placement may let a sewn pair go on a cut whose worst seam
/// is `out_by` centimetres out, in metres.
///
/// Derived, because a number picked to pass would not catch the thing this
/// scene exists for. Where the cloth fits the hoop the map is exact, so the
/// whole of a release gap comes from the pairing walking equal fractions of two
/// unequal sides: the two ends of a pair sit up to `out_by` plus one
/// [`BOUNDARY`] apart in ordinate. The surface may lean at most one band of
/// radius per band of height, so that offset carries into radius at most
/// one for one, and the two together open at most the diagonal of the square.
///
/// Measured: 6.3 mm on the matched cut against 12.7 allowed, and 36.1 mm on
/// his against 42.6 — the worst pair of that one reading dy 23.9 mm, dr 25.6
/// and turn 8.9. One hoop for the whole garment would put about 130 mm here,
/// which is the leg's own narrowing, so the bound has the room it needs to
/// stay a bound.
fn let_go_within(out_by: f64) -> f32 {
    (out_by as f32 / 100.0 + BOUNDARY) * SQRT_2
}

/// Both of a cut's seams, with their two sides measured and judged.
fn judged(doc: &Doc) -> Vec<(Seam, Lengths, f64, Option<bool>)> {
    let draft = Draft::from_doc(doc.clone()).expect("the fixture resolves");
    draft
        .doc()
        .seams
        .iter()
        .map(|(_, seam)| {
            let lengths = Lengths::of(&draft, seam).expect("the fixture anchors all four ends");
            (
                *seam,
                lengths,
                tolerance_cm(&draft, seam),
                lengths.meets(&draft, seam),
            )
        })
        .collect()
}

/// The worst seam of a cut, in centimetres out of its own tolerance.
fn worst_cut(doc: &Doc) -> f64 {
    judged(doc)
        .iter()
        .map(|(seam, lengths, ..)| lengths.excess(seam))
        .fold(0.0, f64::max)
}

/// The two cuts of the leg measure what they were cut to, and the document's
/// own tolerance refuses one of them.
///
/// The reading the owner's jeans needed and nothing in the tree took. His file
/// declares no seams at all, so the judgement that would have told him lived
/// only in the desktop app's panels: a pattern read through the headless door
/// said nothing about either of his two sides. What the mismatch costs is
/// small, and that it is small is not the point — the point is that a person
/// finds out from the document and not from a quarter of an hour of simulation.
#[test]
fn a_leg_cut_with_his_mismatch_is_refused_by_its_own_tolerance() {
    for (what, ease, out_by) in [
        ("matched", Ease::MATCHED, [0.0, 0.0]),
        ("his jeans", Ease::HIS_JEANS, [2.11, 0.72]),
    ] {
        let seams = judged(&leg(ease));
        assert_eq!(seams.len(), 2, "{what}: the leg closes on two seams");
        for (k, (seam, lengths, allowed, meets)) in seams.iter().enumerate() {
            println!(
                "{what} seam {k}: {:.2} cm against {:.2} · out by {:.2} · tolerance {allowed:.2} \
                 · {}",
                lengths.a,
                lengths.b,
                lengths.delta(),
                if *meets == Some(true) {
                    "dentro"
                } else {
                    "fuera"
                }
            );
            assert!(
                (lengths.excess(seam) - out_by[k]).abs() < CUT_TO,
                "{what} seam {k} is cut to {:.2} cm of mismatch, not {:.4}",
                out_by[k],
                lengths.excess(seam)
            );
            assert_eq!(
                *meets,
                Some(out_by[k] <= *allowed),
                "{what} seam {k}: {:.2} cm out against a tolerance of {allowed:.2}",
                out_by[k]
            );
        }
    }
}

/// And a leg cut with his mismatch is let go with its seams all but touching,
/// and worn with them shut, so the mismatch is not what holds a seam open.
///
/// This is the scene that watches the placement. Every number anyone had about
/// his jeans was taken while a sewn product was rolled onto one cylinder, where
/// each piece was laid out by its widest abscissa and every ordinate narrower
/// than that left the difference as open arc at the seams: his two leg pieces
/// were let go 29 to 91 mm apart down one seam and stood 205.7 mm apart at
/// rest. On a surface whose hoop at each ordinate is the cloth really there
/// the worst of their 384 pairs is let go 63 mm apart, 7 mm on the mean, and
/// every one of them shuts to 0.02 mm.
///
/// Read at release and at the mark, never at rest. Nothing holds this garment
/// on, so at rest it has slid down the leg and heaped on the floor, and a pair
/// pinched in a fold of that heap says nothing about either the placement or
/// the cut. That is decision 12 working as written.
#[test]
#[ignore = "release-only: a real body baked and two whole drapes run"]
fn a_leg_cut_with_his_mismatch_is_let_go_and_worn_with_its_seams_shut() {
    let mesh = reference();
    let mut opened = Vec::new();
    for (what, ease) in [("matched", Ease::MATCHED), ("his jeans", Ease::HIS_JEANS)] {
        let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
        let sdf = bake::sdf(&mesh).expect("the Anny body is closed and orientable");
        let doc = leg(ease);
        let out_by = worst_cut(&doc);
        let session = Session::from_doc(doc, body).expect("the leg drapes");
        assert!(
            session.seam_faults().is_empty(),
            "{what}: both seams pair: {:?}",
            session.seam_faults()
        );
        let pairs = session.sewn_pairs();
        let layout = session.layout().expect("the two seams place the leg");
        let start = placed(&session.released());
        let (deep, count) = buried(&sdf, &start);
        println!(
            "{what}: let go at {:.4} · {} pairs · {deep} of {count} buried",
            layout.stand,
            pairs.len()
        );
        assert_eq!(deep, 0, "{what}: {deep} of {count} start under the skin");

        // The placement's own reading, with no simulation run: on a hoop per
        // ordinate the two sides of a seam are let go on the arc their cloth
        // gives them, so what is open here is the cut and nothing else.
        let (worst, _) = report(what, "at release", &pairs, &start).expect("the leg is sewn");
        let allowed = let_go_within(out_by);
        breakdown(what, &pairs, &start, layout.axis);
        assert!(
            worst <= allowed,
            "{what}: the placement let the seams go {:.2} mm apart, against the \
             {:.2} mm a hoop the size of the cloth leaves a cut {out_by:.2} cm out",
            worst * 1000.0,
            allowed * 1000.0
        );
        opened.push(worst);

        let landed = at_mark(&session);
        let (worn, _) = report(what, "at the mark", &pairs, &landed).expect("the leg is sewn");
        println!(
            "{what}: at the mark {} of {count} lie within a cell of the skin",
            touching(&sdf, &landed)
        );
        assert!(
            worn <= HELD_OPEN,
            "{what}: a seam stands further apart than the body can hold it: \
             worst {:.2} mm against {:.1} mm",
            worn * 1000.0,
            HELD_OPEN * 1000.0
        );
    }

    // The mismatch costs something, and this says how much. A fixture whose two
    // cuts were let go alike would not be carrying his mismatch at all, and
    // every assertion above it would be passing on the wrong garment.
    let [matched, his] = [opened[0], opened[1]];
    println!(
        "the cut costs {:.2} mm at release: {:.2} against {:.2}",
        (his - matched) * 1000.0,
        his * 1000.0,
        matched * 1000.0
    );
    assert!(
        his > matched * 2.0,
        "two sides 2.11 cm apart are let go further apart than two of the same \
         length: {:.2} mm against {:.2} mm",
        his * 1000.0,
        matched * 1000.0
    );
}

/// Prints the worst sewn pair of a release taken apart into the three things a
/// gap can be made of: height, distance from the axis, and turn.
///
/// The three are what tells a cut from a placement. A gap that is almost all
/// height is the pairing walking two unequal sides; one that is almost all turn
/// is an arc the cloth never covered.
fn breakdown(what: &str, pairs: &[(u32, u32)], at: &[[f32; 3]], axis: [f32; 2]) {
    let polar = |p: [f32; 3]| {
        let (dx, dz) = (p[0] - axis[0], p[2] - axis[1]);
        (dx.hypot(dz), dx.atan2(dz))
    };
    let Some(&(a, b)) = pairs.iter().max_by(|&&(a, b), &&(c, d)| {
        apart(at[a as usize], at[b as usize]).total_cmp(&apart(at[c as usize], at[d as usize]))
    }) else {
        return;
    };
    let (p, q) = (at[a as usize], at[b as usize]);
    let (ra, ta) = polar(p);
    let (rb, tb) = polar(q);
    println!(
        "  {what}: the worst pair at release is dy {:.2} mm, dr {:.2}, turn {:.2}",
        (q[1] - p[1]).abs() * 1000.0,
        (rb - ra).abs() * 1000.0,
        (tb - ta).abs() * f32::midpoint(ra, rb) * 1000.0
    );
}
