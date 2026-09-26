use std::f64::consts::TAU;

use toile_engine::body::{BodyMesh, Collider, bake};
use toile_engine::couture::{HOLDS_ITS_RATIO, SEAM_SHUT};
use toile_engine::session::Session;
use toile_sim::xpbd::SdfGrid;

use crate::extremes::{GATHERED, wide_hipped};
use crate::skirt::{Cut, cut_to};
use crate::watch::{buried, reference};

/// A published frame's flat triple-per-vertex positions, as points.
pub fn placed(flat: &[f32]) -> Vec<[f32; 3]> {
    flat.as_chunks::<3>().0.to_vec()
}

/// How far apart two placed vertices stand, in metres.
pub fn apart(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [0, 1, 2].map(|k| b[k] - a[k]);
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// How far the two sides of a garment's seams stand apart, in metres: the
/// worst sewn pair and the mean over all of them.
///
/// `None` for a garment with no seams, which is a different answer from zero:
/// a single panel has nothing to close and nothing to say about fit.
pub fn gaps(pairs: &[(u32, u32)], at: &[[f32; 3]]) -> Option<(f32, f32)> {
    let (mut worst, mut sum) = (0.0f32, 0.0f32);
    for &(a, b) in pairs {
        let d = apart(at[a as usize], at[b as usize]);
        worst = worst.max(d);
        sum += d;
    }
    (!pairs.is_empty()).then(|| (worst, sum / pairs.len() as f32))
}

/// Prints that reading and hands it back.
///
/// In millimetres and not in metres, which is the whole of why this is one
/// call and not a `println!` per scene: read in metres a shut seam and a seam
/// a millimetre open both print `0.000`, and the number this suite exists to
/// show is 115.3 mm.
pub fn report(
    scene: &str,
    when: &str,
    pairs: &[(u32, u32)],
    at: &[[f32; 3]],
) -> Option<(f32, f32)> {
    let read = gaps(pairs, at);
    match read {
        Some((worst, mean)) => println!(
            "{scene} seams {when}: {} pairs, worst {:.1} mm, mean {:.1} mm",
            pairs.len(),
            worst * 1000.0,
            mean * 1000.0
        ),
        None => println!("{scene} seams {when}: none — a single panel closes nothing"),
    }
    read
}

/// Reads a scene's seams and holds them to shut.
///
/// [`SEAM_SHUT`] and not a number of this suite's own. What an open seam at
/// rest is worth nobody has decided, and a threshold invented here would be
/// worse than the printed number it replaced — but *shut* is decided, and
/// decided by measurement: it is the distance inside which the solver's own
/// closing phase stops waiting for a seam. Asking the same question the
/// sewing asks is the only form of this assertion that cannot drift from it.
///
/// The worst pair and not the mean. A mean over 353 pairs reads 3.4 mm while
/// one side seam stands 115.3 mm open, so a garment that does not fit passes
/// on the mean; what says a seam met is that every pair of it did.
///
/// # Panics
/// If the scene has no seams, or any pair stands further apart than that.
pub fn shut(scene: &str, when: &str, pairs: &[(u32, u32)], at: &[[f32; 3]]) {
    let Some((worst, _)) = report(scene, when, pairs, at) else {
        panic!("{scene}: held to its seams {when}, and it has none");
    };
    assert!(
        worst <= SEAM_SHUT,
        "{scene}: the seams did not shut {when}: worst pair {:.1} mm apart, \
         against the {:.1} mm the closing phase counts as shut",
        worst * 1000.0,
        SEAM_SHUT * 1000.0
    );
}

/// Bakes a body and lets the skirt drafted to it go, without draping it.
fn let_go(mesh: &BodyMesh, cut: Cut) -> (Session, SdfGrid) {
    let body = Collider::bake(mesh).expect("the Anny body is closed and orientable");
    let sdf = bake::sdf(mesh).expect("the Anny body is closed and orientable");
    let doc = cut_to(cut, Some((GATHERED, HOLDS_ITS_RATIO)));
    (Session::from_doc(doc, body).expect("the skirt drapes"), sdf)
}

/// The ring two bodies' skirts are let go on, and what each one releases open.
///
/// Read and not hashed, deliberately. A `Layout` is four numbers and a wrap
/// per piece, cheap to pin and the wrong instrument twice over: the radius is
/// mostly the body's — `clear_of` walks it out until the person stops
/// swallowing the panels — so a hash would re-pin the bake, which has three
/// goldens already; and the wraps come out of the branch dividing the whole
/// turn between the pieces, one half of a contradiction in `around`'s own
/// doc, so pinning them would price its repair as a golden move.
///
/// What is asserted instead is what the rule promises, and a rule replacing
/// it would still owe: the ring is never smaller round than the cloth,
/// nothing is released already inside the person, and the ring stands within
/// the body's own height rather than over its crown. Which of the body's
/// rings it stands at is sharper and is not asked here — `Collider::belts`
/// is the crate's own, so a test outside it cannot name one.
#[test]
#[ignore = "release-only: two real bodies baked"]
fn the_ring_releases_every_garment_clear_of_the_body() {
    for (scene, mesh, cut) in [
        ("reference adult", reference(), Cut::REFERENCE),
        ("wide-hipped body", wide_hipped(), Cut::WIDE_HIPPED),
    ] {
        let (session, sdf) = let_go(&mesh, cut);
        let (lo, hi) = session.collider().extent();
        let ring = session.layout().expect("the seams place the skirt");
        let round = ring.radius * TAU;
        // The line the ring is sized from. Both halves of this fixture run at
        // their full width from the hip to the hem, so the widest cloth the
        // product carries is twice the cut's own hip, in metres.
        let girth = 2.0 * cut.hip / 100.0;
        let start = placed(&session.released());
        let (deep, all) = buried(&sdf, &start);
        println!(
            "{scene}: ring {round:.5} m round at {:.4}, crest {:.4} · {girth:.5} m of cloth, \
             opened {:.2} cells · body {:.4}..{:.4} · {deep} of {all} released past the band",
            ring.stand,
            ring.crest,
            (round - girth) / (TAU * bake::CELL),
            lo[1],
            hi[1]
        );
        report(scene, "at release", &session.sewn_pairs(), &start);
        assert!(
            round >= girth,
            "{scene}: rolling preserves length, so a ring smaller round than \
             the cloth could only be filled by the pieces overlapping: \
             {round} against {girth}"
        );
        assert_eq!(
            deep, 0,
            "{scene}: the ring released {deep} of {all} particles past the band, \
             where the field is saturated flat and no contact solve has a normal \
             to carry them out along"
        );
        assert!(
            ring.stand > lo[1] && ring.stand < hi[1],
            "{scene}: the garment is hung somewhere down the body and not over \
             its crown: {} against {}..{}",
            ring.stand,
            lo[1],
            hi[1]
        );
    }
}
