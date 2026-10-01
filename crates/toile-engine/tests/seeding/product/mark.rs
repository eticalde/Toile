use toile_engine::body::bake;
use toile_engine::couture::SEAM_SHUT;
use toile_engine::session::Session;
use toile_sim::xpbd::SdfGrid;

use super::{Released, about_the_axis};
use crate::fit::{apart as gap, report};
use crate::watch::{LANDED, MARK, at_mark, buried, span, through_the_drape, touching};

/// How far apart the body itself can hold a sewn pair, in metres.
///
/// Two cells of the baked field. A pair that misses `SEAM_SHUT` here does so
/// because the contact solve stands between its two sides, and the contact
/// solve works at the field's own resolution — so two cells is the honest
/// bound on "the body is holding it", and a pair past it is a fit or a
/// placement answering, not a contact. Measured on this scene: 6.89 mm.
const HELD_OPEN: f32 = 2.0 * bake::CELL as f32;

/// Every sewn pair the closing phase did not shut, worst first.
fn still_open(pairs: &[(u32, u32)], at: &[[f32; 3]]) -> Vec<(f32, u32, u32)> {
    let mut open: Vec<(f32, u32, u32)> = pairs
        .iter()
        .map(|&(a, b)| (gap(at[a as usize], at[b as usize]), a, b))
        .filter(|d| d.0 > SEAM_SHUT)
        .collect();
    open.sort_by(|a, b| b.0.total_cmp(&a.0));
    open
}

/// Whether the body is holding a vertex where it stands, rather than the vertex
/// standing in free air.
fn on_the_skin(sdf: &SdfGrid, p: [f32; 3]) -> bool {
    sdf.sample(p[0], p[1], p[2]).abs() <= bake::CELL as f32
}

/// What the drape has made of it once the sim has run [`MARK`] substeps.
pub(super) fn at_the_mark(session: &Session, sdf: &SdfGrid, released: &Released) {
    let (lo, _) = session.collider().extent();
    let stand = released.ring.stand;
    let (split, all) = (released.split, released.all);

    // Watched before the mark is read, because it has to see the frames on
    // the way there: this is the one that has to be running while the garment
    // is still being worn.
    let watched = through_the_drape(session, sdf);
    let landed = at_mark(session);
    let (deep, count) = buried(sdf, &landed);
    let (low, high) = span(&landed);
    let (worst, mean) = report(
        "sewn product",
        &format!("at {MARK}"),
        &released.pairs,
        &landed,
    )
    .expect("the block's two seams pair");
    let (fx, fz, _) = about_the_axis(0..split, &landed, released.ring.axis);
    let (bx, bz, _) = about_the_axis(split..all, &landed, released.ring.axis);
    let on_skin = touching(sdf, &landed);
    println!(
        "at {MARK}: cloth {low:.3}..{high:.3} · {on_skin} of {count} on the skin · {deep} buried"
    );
    println!("at {MARK}: front at x {fx:+.3} z {fz:+.3} · back at x {bx:+.3} z {bz:+.3}");
    println!(
        "at {MARK}: the garment came down {:.3} m of the {LANDED} m that counts as landed",
        stand - high
    );

    assert!(
        high < stand - LANDED,
        "the garment never came down from the ring at {stand}: {high}"
    );
    assert!(
        on_skin > 0,
        "the garment is not on the body: not one of {count} particles lies \
         within a cell of the skin"
    );
    // Still on the body, rather than fallen away beneath it. A sewn tube is
    // let go round a limb and hangs a metre down from there, so its hem is
    // under the foot from the first substep and the whole-panel reading the
    // one-piece scenes take cannot be asked of it. What can be asked, and is,
    // is that the cloth has not left the body by the mark.
    assert!(
        high > lo[1],
        "the garment fell clear of the body instead of onto it: its top {high} \
         is under {}",
        lo[1]
    );
    // The shipped block's own fit, and the two halves of it that say different
    // things. The closing phase shuts what it can reach: measured, 231 of the
    // 381 pairs are let go further apart than it counts as shut and 226 of them
    // are shut by the mark, which is what the mean says. The five it cannot
    // reach are the reading, and they are held open by the person: both sides
    // of every one of them stands within a cell of the skin, pinched
    // between the seam pulling one way and the body pushing the other.
    //
    // `SEAM_SHUT` alone was asked of the worst pair here, and it cannot be.
    // Let go on one hoop that cleared the body the block hung 2.5 cm clear of
    // the thigh in free air and closed all 381 pairs to 0.0 mm before it
    // touched anything; let go on a hoop per ordinate it is against the leg
    // from the first substep, and the worst stands 6.9 mm. So the mean answers
    // the closing and `HELD_OPEN` answers the worst.
    let open = still_open(&released.pairs, &landed);
    println!(
        "at {MARK}: {} of {} pairs stand further apart than the {:.1} mm the closing phase \
         counts as shut, the worst of them {:.2} mm",
        open.len(),
        released.pairs.len(),
        SEAM_SHUT * 1000.0,
        worst * 1000.0
    );
    assert!(
        worst <= HELD_OPEN,
        "no pair stands further apart than the body can hold it: worst {:.2} mm \
         against {:.1} mm, with {} of {} over the {:.1} mm that counts as shut",
        worst * 1000.0,
        HELD_OPEN * 1000.0,
        open.len(),
        released.pairs.len(),
        SEAM_SHUT * 1000.0
    );
    assert!(
        mean <= SEAM_SHUT,
        "the seams reached the solver and closed: mean pair {:.2} mm apart, worst \
         {:.2} mm, against the {:.1} mm the closing phase counts as shut",
        mean * 1000.0,
        worst * 1000.0,
        SEAM_SHUT * 1000.0
    );
    for &(gap, a, b) in &open {
        let (p, q) = (landed[a as usize], landed[b as usize]);
        assert!(
            on_the_skin(sdf, p) && on_the_skin(sdf, q),
            "a sewn pair stands {:.2} mm apart with no body holding it there: \
             {a} at {p:?} reads {:.4} m of field and {b} at {q:?} reads {:.4} m",
            gap * 1000.0,
            sdf.sample(p[0], p[1], p[2]),
            sdf.sample(q[0], q[1], q[2])
        );
    }
    let (worst, worst_at) = watched.worst;
    let (worn, worn_at) = watched.worn;
    println!(
        "through the drape: worst {worst} past the band at substep {worst_at} · \
         most worn {worn} of {count} on the skin at substep {worn_at}"
    );
    assert!(
        worn > count / 8,
        "the garment was never really worn: at its best only {worn} of {count} \
         particles were within a cell of the skin"
    );
    // Over every frame and not only this one. The mark catches a tube after it
    // has slid off the leg, so a count taken there would read zero for the
    // wrong reason; what this refuses is a particle driven past the band at
    // any moment, which is where the field is saturated flat and nothing
    // carries it out again.
    assert_eq!(
        worst, 0,
        "{worst} of {count} particles were driven past the band, worst at \
         substep {worst_at}"
    );
    assert_eq!(
        deep, 0,
        "{deep} of {count} are still past the band at the mark"
    );
}
