use std::f64::consts::TAU;

use strip::{Step, adjacency, walk};

use super::sew::Sewn;
use crate::body::{Collider, bake, worn_at};
use crate::couture::{Layout, ShapePipeline, Wrap};

mod strip;

/// How far the ring is opened at a time when the body needs the room, in
/// metres.
///
/// The field's own cell. A step finer asks the field a question it cannot
/// answer differently, and one coarser hands the garment room it never needed.
const OPENING: f64 = bake::CELL;

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
/// and say which way round each one runs. The cloth says how big the ring is:
/// a garment is the size of the cloth it is cut from, never the size of the
/// person. The body says where on itself a ring that size belongs, and how far
/// the ring has to be opened to go round it at all.
///
/// A piece's arc is its own flat width, so nothing is stretched to fit, and a
/// closed garment divides the whole turn between its pieces.
///
/// `None` when the seams do not chain the pieces into one strip — a piece sewn
/// to itself, one carrying three seams, or two strips at once. The placement
/// those want is not a turn around one axis, and guessing one would be
/// inventing the very thing this reads.
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

    // The garment's own size first, opened only as far as the person under it
    // makes necessary. A ring smaller than the body starts the cloth inside it,
    // past the band where the field is saturated flat and no contact solve has
    // a normal to carry it out along.
    let far = outermost(collider, axis);
    let mut radius = girth / TAU;
    loop {
        let on = Ring {
            axis,
            radius,
            stand,
            crest,
        };
        let layout = rolled(&steps, pipes.len(), girth, closed, &on);
        if radius >= far || clear_of(&layout, pipes, collider) {
            return Some(layout);
        }
        radius += OPENING;
    }
}

/// The cylinder a candidate placement stands on.
struct Ring {
    axis: [f32; 2],
    radius: f64,
    stand: f32,
    crest: f64,
}

/// Rolls the strip onto one candidate ring.
///
/// The first piece's middle opens the turn, so a product comes out facing the
/// same way whatever ring it ends on.
fn rolled(steps: &[Step], pieces: usize, girth: f64, closed: bool, on: &Ring) -> Layout {
    let mut along = Vec::with_capacity(steps.len());
    let mut walked = 0.0;
    for step in steps {
        let width = step.hi - step.lo;
        along.push(walked + width / 2.0);
        walked += width;
    }
    // A closed garment divides the whole turn between its pieces; an open one
    // has no whole to divide, so its pieces simply abut at the ring's own size.
    let turn_of = |mid: f64| {
        if closed {
            TAU * mid / girth
        } else {
            mid / on.radius
        }
    };
    let origin = turn_of(along[0]);
    let mut wraps = vec![None; pieces];
    for (k, step) in steps.iter().enumerate() {
        let middle = f64::midpoint(step.lo, step.hi);
        wraps[step.piece] = Some(Wrap {
            turn: turn_of(along[k]) - origin - step.sense * middle / on.radius,
            sense: step.sense,
        });
    }
    Layout {
        axis: on.axis,
        radius: on.radius,
        stand: on.stand,
        crest: on.crest,
        wraps,
    }
}

/// Whether no part of the rolled cloth is anywhere the body has swallowed it.
fn clear_of(layout: &Layout, pipes: &[&ShapePipeline], collider: &Collider) -> bool {
    pipes.iter().enumerate().all(|(at, pipe)| {
        pipe.pos2d
            .iter()
            .all(|&p| layout.point(at, p).is_none_or(|q| !collider.swallows(q)))
    })
}

/// Where the ring sits: the body's own ring for a garment this size, or the
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
