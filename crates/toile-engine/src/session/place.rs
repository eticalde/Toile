use strip::{Step, adjacency, walk};

use super::sew::Sewn;
use crate::body::{Collider, bake, worn_at};
use crate::couture::{Layout, Round, ShapePipeline};

mod strip;

/// How far the surface is opened at a time where the body needs the room, in
/// metres.
///
/// The field's own cell. A step finer asks the field a question it cannot
/// answer differently, and one coarser hands the garment room it never needed.
const OPENING: f64 = bake::CELL;

/// How tall one band of the clearance profile is, in metres.
///
/// The field's own cell again, and for the same reason: two bands closer
/// together than that would be opened by a reading the field cannot tell apart.
/// It is also what makes the profile's own slope readable — a band of height
/// per band of radius is the steepest wall the surface is allowed to lean at.
const BAND_HEIGHT: f64 = bake::CELL;

/// How far round a product's elastics go, and the line of cloth they run at.
///
/// What holds a garment on is what says where it hangs from, so a product
/// that carries elastics is placed by them: see [`around`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Band {
    /// The cloth the elastics cover, across every piece, in metres.
    pub(super) girth: f64,
    /// The pattern ordinate they run at, in metres.
    pub(super) at: f64,
}

/// Where a sewn product is let go on the body.
///
/// Three readings, from three places. The seams chain the pieces into a strip
/// and say which way round each one runs. The cloth says how big the surface is
/// at every height. The body says where on itself a garment that size belongs,
/// and which of those heights have to be opened to go round it at all.
///
/// One radius cannot do the middle reading. A skirt's waistband carries 61 cm
/// of cloth and its hip 117, and the one hoop that cleared the person was
/// 130 cm round: the band — the line that has to close first and holds the
/// garment up — was let go at 47 % coverage, its two ends most of its own
/// length apart. A hoop per ordinate lands every line of cloth on a hoop its
/// own size, so the arc a seam walks is the arc its cloth gives it.
///
/// `None` when the seams do not chain the pieces into one strip — a piece sewn
/// to itself, one carrying three seams, or two at once. What those want is not
/// a turn around one axis, and guessing one would invent what this reads.
pub(super) fn around(
    sewn: &[Sewn],
    pipes: &[&ShapePipeline],
    collider: &Collider,
    band: Option<Band>,
) -> Option<Layout> {
    let joins = adjacency(sewn, pipes.len())?;
    let (steps, closed) = walk(sewn, pipes, &joins)?;
    if steps.len() != joins.iter().filter(|on| !on.is_empty()).count() {
        return None;
    }
    if steps.iter().any(|s| s.hi - s.lo <= f64::EPSILON) {
        return None;
    }
    // Across the cloth, never between the seams' middles. A seam runs down the
    // edge of a piece and the cloth reaches past it to the piece's own edge, so
    // measuring middle to middle loses everything outside the two seams — a
    // fifth of this block's trousers, which would then wrap a ring a fifth too
    // small and overlap each other on it.
    let girth: f64 = steps.iter().map(|s| s.hi - s.lo).sum();
    let (top, hem) = rise(&steps, pipes);
    // A garment is held up by one line of itself, so that line is the one
    // matched to a ring and the one stood on it. With an elastic that is the
    // elastic; without one it is the topmost edge, as it always was. Read the
    // widest girth for both and a tube skirt is matched by its hip to the ring
    // nearest that — the upper chest, measured — and then stood with its
    // waistline up there, two rings above where a waistband belongs.
    let (hangs_by, crest) = band.map_or((girth, top), |band| (band.girth, band.at));
    let (axis, stand) = stands(collider, hangs_by, girth, crest - hem);

    // The garment's own size first, opened only at the heights the person under
    // it makes necessary. Cloth let go inside the body starts past the band
    // where the field is saturated flat and no contact solve has a normal to
    // carry it out along.
    let far = outermost(collider, axis);
    let order: Vec<(usize, f64)> = steps.iter().map(|s| (s.piece, s.sense)).collect();
    let round = Round::over(&order, pipes, closed, BAND_HEIGHT)?;
    let mut layout = Layout::on(round, axis, stand, crest, pipes.len());
    // Band by band rather than the whole surface at once, which is the point:
    // a garment that fits the person over most of its height is opened only
    // where it does not, and everywhere else the hoops stay the size of the
    // cloth and the seams meet. The walk cannot circle — every round either
    // stops or opens a band it has not yet walked out to `far`.
    while layout.open(&swallowed(&layout, pipes, collider), OPENING, far) {}
    Some(layout)
}

/// Every pattern ordinate at which the body has swallowed some of the rolled
/// cloth.
///
/// The ordinates and not the vertices, because what is opened is a height: a
/// panel caught at the hip says the hip's hoop is too small, and says nothing
/// at all about the waistband two hand's breadths above it.
///
/// At the band and not a step short of it, which was measured and costs the
/// whole point of the rule. Stopping the walk one opening early does give the
/// release a real margin — 5.2 mm and 5.7 mm on the two skirts, against the
/// 51 µm and 12 µm `fit.rs` reads — and it shuts the shipped block's seams
/// besides, 1 pair of 381 standing open at the mark instead of 5. It pays for
/// that at the one place that must not pay: the tightest hoop in a garment is
/// the line it hangs by, so the waistband is the first band a margin opens,
/// and the reference skirt's band comes off a hoop 3.5 % longer than its own
/// cloth with the held run stretched 2.87 % — a garment let go too big at the
/// band, which is the defect the ordinate-by-ordinate hoop exists to end.
fn swallowed(layout: &Layout, pipes: &[&ShapePipeline], collider: &Collider) -> Vec<f64> {
    let mut inside = Vec::new();
    for (at, pipe) in pipes.iter().enumerate() {
        for &p in &pipe.pos2d {
            if layout.point(at, p).is_some_and(|q| collider.swallows(q)) {
                inside.push(p[1]);
            }
        }
    }
    inside
}

/// Where the surface sits: the body's own ring for a garment this size, or the
/// middle of the body's box at the height it releases from.
///
/// The fallback is for a body that carries no measurements — the demo ball,
/// and the cube a test bakes. There is no ring to be matched to, so the
/// garment is let go about the whole of it, which is where every garment went
/// before any ring was read.
fn stands(collider: &Collider, hangs_by: f64, widest: f64, rise: f64) -> ([f32; 2], f32) {
    if let Some(belt) = worn_at(collider.belts(), hangs_by, widest, rise) {
        return (belt.centre, belt.height);
    }
    let (lo, hi) = collider.extent();
    (
        [f32::midpoint(lo[0], hi[0]), f32::midpoint(lo[2], hi[2])],
        collider.release_height(),
    )
}

/// The radius past which none of the body is left to clear: from the axis to
/// the furthest corner of the body's own box.
///
/// The opening stops here whatever the field says, so a garment the field
/// never reports clear cannot be walked outwards for ever.
fn outermost(collider: &Collider, axis: [f32; 2]) -> f64 {
    let (lo, hi) = collider.extent();
    let reach = |far: f32, near: f32, from: f32| {
        f64::from(far - from)
            .abs()
            .max(f64::from(from - near).abs())
    };
    let x = reach(hi[0], lo[0], axis[0]);
    let z = reach(hi[2], lo[2], axis[1]);
    (x * x + z * z).sqrt()
}

/// The highest and lowest pattern ordinate over the pieces a walk places.
fn rise(steps: &[Step], pipes: &[&ShapePipeline]) -> (f64, f64) {
    steps.iter().fold((f64::MIN, f64::MAX), |(hi, lo), step| {
        pipes[step.piece]
            .pos2d
            .iter()
            .fold((hi, lo), |(h, l), p| (h.max(p[1]), l.min(p[1])))
    })
}
