use std::f32::consts::PI;

use toile_engine::body::{Collider, bake};
use toile_engine::couture::Layout;
use toile_engine::draft::{PieceKey, block};
use toile_engine::session::Session;
use toile_sim::xpbd::SdfGrid;

use crate::watch::{LANDED, MARK, at_mark, buried, reference, span, through_the_drape, touching};

/// How far from a seam, in metres of cloth, a vertex has to be before the two
/// pieces coming close there counts as one passing through the other.
///
/// Near a seam the two pieces are *meant* to meet, so the smallest gap in the
/// product is always a sewn one and says nothing. Cloth distance is taken on
/// the release state, where each piece is a rigid rolled panel and a straight
/// line across it is the cloth's own length.
const AWAY: f32 = 0.05;

/// What the release measured, and what reading the mark needs from it.
struct Released {
    /// Every sewn pair, as indices into the published positions.
    pairs: Vec<(u32, u32)>,
    /// Each piece's vertices that lie more than [`AWAY`] of cloth from any
    /// seam: the ones whose meeting would be one piece inside the other.
    clear: [Vec<usize>; 2],
    /// Where the back's block of the combined state begins, and how long the
    /// whole of it is.
    split: usize,
    all: usize,
    /// The ring the product was let go on.
    ring: Layout,
}

/// The shipped block over the reference body: two pieces, two seams.
fn trousers() -> (Session, SdfGrid, [PieceKey; 2]) {
    let mesh = reference();
    let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    let sdf = bake::sdf(&mesh).expect("the Anny body is closed and orientable");
    let session = Session::from_doc(block::trousers(), body).expect("the block drapes");
    let doc = session.draft().expect("the block has a document").doc();
    let front = doc
        .piece_named(block::FRONT)
        .expect("the block draws a front");
    let back = doc
        .piece_named(block::BACK)
        .expect("the block draws a back");
    (session, sdf, [front, back])
}

fn points(flat: &[f32]) -> Vec<[f32; 3]> {
    flat.as_chunks::<3>().0.to_vec()
}

fn gap(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt()
}

/// Largest and mean separation of the sewn pairs.
fn seam_gaps(pairs: &[(u32, u32)], at: &[[f32; 3]]) -> (f32, f32) {
    let (mut worst, mut sum) = (0.0f32, 0.0f32);
    for &(a, b) in pairs {
        let d = gap(at[a as usize], at[b as usize]);
        worst = worst.max(d);
        sum += d;
    }
    (worst, sum / pairs.len() as f32)
}

/// The vertices of one run that are at least [`AWAY`] of cloth from any seam.
fn off_the_seams(run: std::ops::Range<usize>, sewn: &[usize], at: &[[f32; 3]]) -> Vec<usize> {
    run.filter(|&i| sewn.iter().all(|&s| gap(at[i], at[s]) > AWAY))
        .collect()
}

/// Closest approach between two sets of vertices.
fn nearest(a: &[usize], b: &[usize], at: &[[f32; 3]]) -> f32 {
    let mut best = f32::MAX;
    for &i in a {
        for &j in b {
            best = best.min(gap(at[i], at[j]));
        }
    }
    best
}

/// Where a run of the product sits around the ring's axis, in metres.
fn about_the_axis(run: std::ops::Range<usize>, at: &[[f32; 3]], axis: [f32; 2]) -> (f32, f32, f32) {
    let n = run.len() as f32;
    let (mut x, mut z, mut r) = (0.0f32, 0.0f32, 0.0f32);
    for i in run {
        let (dx, dz) = (at[i][0] - axis[0], at[i][2] - axis[1]);
        x += dx;
        z += dz;
        r += dx.hypot(dz);
    }
    (x / n, z / n, r / n)
}

/// A product of two sewn pieces is let go on the part of the body it belongs
/// to, at its own size, and the drape pulls its seams shut without driving any
/// of it into the person.
///
/// The ring is the garment's: the cloth the two pieces carry across, opened
/// only as far as the body under it makes necessary. Where it sits is the
/// body's: the measurement ring a garment this size is worn at, which for one
/// leg's worth of trouser is the thigh.
///
/// Read at a fixed substep and not at rest, deliberately. Since the floor
/// landed a sewn tube does come to a stop — at about 12,200 substeps — but it
/// stops in a heap on the ground. What this asks is how the product sits *on*
/// the body while it is still being worn, and only the mark answers that.
#[test]
#[ignore = "release-only: a real body baked and a whole drape run"]
fn a_sewn_product_is_let_go_around_the_body() {
    let (session, sdf, [front, back]) = trousers();
    assert_eq!(session.pieces(), [front, back], "both pieces drape");
    assert!(
        session.seam_faults().is_empty(),
        "the block's seams all pair: {:?}",
        session.seam_faults()
    );
    let released = let_go(&session, &sdf, back);
    at_the_mark(&session, &sdf, &released);
}

/// Where the seams and the body put the two pieces.
fn let_go(session: &Session, sdf: &SdfGrid, back: PieceKey) -> Released {
    let (lo, hi) = session.collider().extent();
    let ring = session.layout().expect("the seams place the product");
    let axis = ring.axis;
    let split = session.offset(back).expect("the back drapes") as usize;
    let all = session.n_vertices();
    let pairs = session.sewn_pairs();
    assert_eq!(pairs.len(), 381, "the two seams pair every boundary vertex");

    let start = points(&session.released());
    let (deep, count) = buried(sdf, &start);
    let (low, high) = span(&start);
    let (fx, fz, fr) = about_the_axis(0..split, &start, axis);
    let (bx, bz, br) = about_the_axis(split..all, &start, axis);
    println!(
        "body {:.3}..{:.3} x, {:.3}..{:.3} y, {:.3}..{:.3} z",
        lo[0], hi[0], lo[1], hi[1], lo[2], hi[2]
    );
    println!(
        "ring: axis {axis:?} · radius {:.4} m ({:.3} m round) · height {:.4}",
        ring.radius,
        ring.radius * std::f64::consts::TAU,
        ring.stand
    );
    println!(
        "release: cloth {low:.3}..{high:.3} · front at x {fx:+.3} z {fz:+.3} r {fr:.3} · \
         back at x {bx:+.3} z {bz:+.3} r {br:.3} · {deep} of {count} buried"
    );
    assert_eq!(deep, 0, "{deep} of {count} particles start under the skin");
    assert!(
        fz > 0.0 && bz < 0.0,
        "the seams put one piece before the body and one behind it: {fz} and {bz}"
    );
    let turn = (bz.atan2(bx) - fz.atan2(fx)).abs();
    println!(
        "turn between the two pieces {:.1}°",
        turn.to_degrees().min(360.0 - turn.to_degrees())
    );
    assert!(
        (turn - PI).abs() < 0.35,
        "and puts them across the axis from each other: {turn} rad apart"
    );

    let (worst, mean) = seam_gaps(&pairs, &start);
    println!("release: sewn pairs {mean:.3} m apart on average, worst {worst:.3} m");
    // The ring is the cloth's own size, so what is left open at release is the
    // shape of the pieces and not the size of the body: the two seams are a
    // hand's breadth apart, never a turn of the whole body away.
    assert!(
        mean < 0.15,
        "the pieces are let go all but touching: {mean} m apart on average"
    );

    // Cloth distance to the nearest seam, read off the release state, so that
    // the pieces closing on each other at the seam is not mistaken later for
    // one of them passing through the other.
    let sewn: Vec<usize> = pairs
        .iter()
        .flat_map(|&(a, b)| [a as usize, b as usize])
        .collect();
    let clear = [
        off_the_seams(0..split, &sewn, &start),
        off_the_seams(split..all, &sewn, &start),
    ];
    println!(
        "{} of {split} front and {} of {} back vertices lie more than {AWAY} m of cloth from a seam",
        clear[0].len(),
        clear[1].len(),
        all - split
    );
    let apart = nearest(&clear[0], &clear[1], &start);
    println!("release: the two pieces come no closer than {apart:.3} m away from their seams");
    assert!(
        apart > AWAY,
        "the pieces are rolled onto disjoint arcs of one ring, so at release \
         no part of one can be inside the other: {apart} m"
    );
    Released {
        pairs,
        clear,
        split,
        all,
        ring,
    }
}

/// What the drape has made of it once the sim has run [`MARK`] substeps.
fn at_the_mark(session: &Session, sdf: &SdfGrid, released: &Released) {
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
    let (worst, mean) = seam_gaps(&released.pairs, &landed);
    let (fx, fz, _) = about_the_axis(0..split, &landed, released.ring.axis);
    let (bx, bz, _) = about_the_axis(split..all, &landed, released.ring.axis);
    let on_skin = touching(sdf, &landed);
    println!(
        "at {MARK}: cloth {low:.3}..{high:.3} · {on_skin} of {count} on the skin · {deep} buried"
    );
    println!(
        "at {MARK}: sewn pairs {mean:.3} m apart on average, worst {worst:.3} m · \
         front at x {fx:+.3} z {fz:+.3} · back at x {bx:+.3} z {bz:+.3}"
    );
    println!(
        "at {MARK}: the two pieces come no closer than {:.3} m away from their seams",
        nearest(&released.clear[0], &released.clear[1], &landed)
    );
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
    assert!(
        mean < 0.005 && worst < 0.05,
        "the seams reached the solver and closed: {mean} mean, {worst} worst"
    );
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
