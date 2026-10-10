use super::Elsewhere;
use crate::body::{Belt, Collider, at_station, station_of, worn_at};

/// Where the surface sits: the ring the document named, the body's own ring for
/// a garment this size, or the middle of the body's box.
///
/// In that order, and the order is the whole rule: the declaration wins where
/// there is one, and the inference stays underneath it as the backstop. It is
/// not replaced, because the question it answers is still a good one — a
/// trouser leg carries a thigh's worth of cloth and this body's head is within
/// a centimetre of its thigh, so a garment nobody placed still has to be
/// stopped from being hung off the crown.
///
/// The last fallback is for a body that carries no measurements — the demo
/// ball, and the cube a test bakes. There is no ring to be matched to, so the
/// garment is let go about the whole of it, which is where every garment went
/// before any ring was read.
pub(super) fn stands(
    collider: &Collider,
    station: Option<&str>,
    hangs_by: f64,
    widest: f64,
    rise: f64,
) -> ([f32; 2], f32, Option<Elsewhere>) {
    let belts = collider.belts();
    let inferred = worn_at(belts, hangs_by, widest, rise);
    if let Some(station) = station
        && let Some(belt) = at_station(belts, station)
    {
        return (belt.centre, belt.height, apart(belts, station, inferred));
    }
    if let Some(belt) = inferred {
        return (belt.centre, belt.height, None);
    }
    let (lo, hi) = collider.extent();
    (
        [f32::midpoint(lo[0], hi[0]), f32::midpoint(lo[2], hi[2])],
        collider.release_height(),
        None,
    )
}

/// The two rings, when the body's own reading would not have chosen the one the
/// document declared; nothing at all when they agree.
///
/// Quiet on agreement, which is the ordinary case: a garment drafted to the
/// person it is hung on reads the same ring both ways, and a box that lit up on
/// every one of those would be read for a week and then never again.
fn apart(belts: &[Belt], declared: &str, inferred: Option<&Belt>) -> Option<Elsewhere> {
    let named = station_of(belts, inferred?)?;
    (named != declared).then(|| Elsewhere {
        declared: declared.to_owned(),
        inferred: named.to_owned(),
    })
}
