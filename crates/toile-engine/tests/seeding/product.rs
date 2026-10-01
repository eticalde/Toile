use std::f32::consts::PI;

use toile_engine::body::{Collider, bake};
use toile_engine::couture::Layout;
use toile_engine::draft::{PieceKey, block};
use toile_engine::session::Session;
use toile_sim::xpbd::SdfGrid;

use crate::fit::{placed as points, report};
use crate::watch::{buried, reference, span};

/// Where the two pieces sit round the surface: one arc each.
mod arcs;
/// What the garment has become once the sim has run to the mark.
mod mark;
/// One piece's meshed boundary, read by this suite rather than by the engine.
mod outline;
/// What the surface does above the height the shortest piece stops at.
mod tab;

/// What the release measured, and what reading the mark needs from it.
pub(super) struct Released {
    /// Every sewn pair, as indices into the published positions.
    pub(super) pairs: Vec<(u32, u32)>,
    /// Where the back's block of the combined state begins, and how long the
    /// whole of it is.
    pub(super) split: usize,
    pub(super) all: usize,
    /// The ring the product was let go on.
    pub(super) ring: Layout,
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

/// Where a run of the product sits around the ring's axis, in metres.
pub(super) fn about_the_axis(
    run: std::ops::Range<usize>,
    at: &[[f32; 3]],
    axis: [f32; 2],
) -> (f32, f32, f32) {
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
/// Read at a fixed substep and not at rest, deliberately. A sewn tube does
/// come to a stop, but not at the ring it was let go on: nothing yet holds a
/// garment where it was put, so at rest it is further down the body or on the
/// ground. What this asks is how the product sits *on* the body while it is
/// still being worn, and only the mark answers that.
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
    mark::at_the_mark(&session, &sdf, &released);
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

    let (_, mean) =
        report("sewn product", "at release", &pairs, &start).expect("the block's two seams pair");
    // The ring is the cloth's own size, so what is left open at release is the
    // shape of the pieces and not the size of the body: the two seams are a
    // hand's breadth apart, never a turn of the whole body away.
    assert!(
        mean < 0.15,
        "the pieces are let go all but touching: {mean} m apart on average"
    );

    // Where the two pieces sit round the surface, and what the surface does
    // above the height the front stops at. Both are readings of this very
    // release, so they are taken here rather than from a scene of their own.
    arcs::one_arc_each(session, &ring);
    tab::above_the_strip(session, &ring);
    Released {
        pairs,
        split,
        all,
        ring,
    }
}
