use toile_anny::asset::RingId;
use toile_anny::measure;

use super::BodyMesh;

/// One of the body's own measurement rings, as a placement reads it.
///
/// The ring itself is a loop of points on the skin; what putting a garment
/// somewhere needs of it is only how far round it goes, how high it sits and
/// where its middle is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Belt {
    /// How far round the ring goes, in metres.
    pub girth: f64,
    /// How high it sits, in metres.
    pub height: f32,
    /// Its own middle: x, then z, in metres.
    ///
    /// Not the middle of the body's box. A body stands with its arms out and
    /// its feet apart, so that box is centred eleven centimetres behind the
    /// waist on this body and half a metre wide of either thigh.
    pub centre: [f32; 2],
}

/// Every ring of the body, in the asset's own order.
///
/// Empty for a mesh the rings were not cut from — the demo ball, or the cube
/// a test bakes. A ring is a fixed run of vertex indices into Anny's own
/// layout, so against any other mesh it would quietly measure whatever
/// vertices happened to carry those numbers.
pub(super) fn of(mesh: &BodyMesh) -> Vec<Belt> {
    if mesh.positions.len() != measure::BODY_VERTEX_COUNT * 3 {
        return Vec::new();
    }
    RingId::ALL
        .iter()
        .map(|&id| {
            let middle = measure::centroid(&mesh.positions, measure::ring(id).points);
            Belt {
                girth: f64::from(measure::girth(&mesh.positions, id)),
                height: middle[1],
                centre: [middle[0], middle[2]],
            }
        })
        .collect()
}

/// Which ring a garment `girth` metres round and `rise` metres deep is worn
/// at; `None` for a body that carries no rings.
///
/// Two questions, in this order. First, can the garment be there at all: it
/// is put on over the body and then hangs down from where it sits, so every
/// ring it would have to pass on the way down, within its own rise, has to be
/// no wider than the garment is. That is what stops a trouser leg from being
/// hung round the head, which its girth alone would happily accept — the head
/// and the thigh of this body are within a centimetre of each other, and the
/// chest below the head is not.
///
/// Then, of the rings that are left, the one nearest its girth. Nearest as a
/// ratio and not a difference, so the same rule reads a cuff and a coat: a
/// centimetre is most of a wrist and nothing on a hip.
pub(crate) fn worn_at(belts: &[Belt], girth: f64, rise: f64) -> Option<&Belt> {
    belts
        .iter()
        .filter(|belt| passable(belts, belt, girth, rise))
        .min_by(|a, b| apart(a, girth).total_cmp(&apart(b, girth)))
}

/// Whether nothing the garment would hang over is wider than the garment.
fn passable(belts: &[Belt], belt: &Belt, girth: f64, rise: f64) -> bool {
    let hem = belt.height - rise as f32;
    !belts
        .iter()
        .any(|under| under.height < belt.height && under.height >= hem && under.girth > girth)
}

/// How far a ring is from a garment's girth, as a ratio at or above one.
///
/// Reciprocal rather than a logarithm because the same answer is wanted from
/// `+ - * /` alone: a ring twice the garment and one half of it are equally
/// far from it, and neither reading needs a transcendental.
fn apart(belt: &Belt, girth: f64) -> f64 {
    let ratio = girth / belt.girth;
    if ratio >= 1.0 { ratio } else { 1.0 / ratio }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "a ring hands back the very girth it was built with"
    )]

    use super::*;

    /// Heights and girths in the order a body has them, top down.
    fn body() -> Vec<Belt> {
        [
            (0.983, 0.592),  // head
            (0.585, 1.048),  // upper chest
            (0.406, 0.880),  // waist
            (0.204, 1.030),  // hip
            (0.054, 0.580),  // thigh
            (-0.274, 0.376), // knee
            (-0.693, 0.253), // ankle
        ]
        .map(|(height, girth)| Belt {
            girth,
            height,
            centre: [0.0, 0.0],
        })
        .to_vec()
    }

    /// A trouser leg carries seventy centimetres of cloth and hangs a metre.
    /// Its girth alone would put it on the head, which is within a centimetre
    /// of the thigh; the chest under the head is what rules the head out.
    #[test]
    fn a_leg_of_cloth_is_worn_at_the_thigh_and_not_at_the_head() {
        let body = body();
        let worn = worn_at(&body, 0.7025, 1.065).expect("the body carries rings");
        assert_eq!(worn.girth, 0.580);
        assert_eq!(worn.height, 0.054);
    }

    /// Where the rule runs out. Girth cannot tell a long tube from a trouser,
    /// so a garment wider than every ring it can reach lands on the widest of
    /// them — here the chest, for cloth that is twice a leg's. This is the
    /// limit and not an accident: what would settle it is which part of the
    /// body a piece belongs to, and no pattern in this tree says.
    #[test]
    fn a_garment_wider_than_every_ring_lands_on_the_widest_it_can_reach() {
        let body = body();
        let worn = worn_at(&body, 1.405, 1.065).expect("the body carries rings");
        assert_eq!(worn.girth, 1.048);
    }

    /// A body with no rings names none, rather than guessing at one.
    #[test]
    fn a_body_without_rings_is_worn_nowhere() {
        assert_eq!(worn_at(&[], 0.7, 1.0), None);
    }

    /// Nearness is a ratio: a ring half the garment's girth and one twice it
    /// are the same distance away, so no size of garment is read differently
    /// from any other.
    #[test]
    fn nearness_is_a_ratio_and_not_a_difference() {
        let belt = |girth| Belt {
            girth,
            height: 0.0,
            centre: [0.0, 0.0],
        };
        assert!((apart(&belt(0.5), 1.0) - 2.0).abs() < 1.0e-12);
        assert!((apart(&belt(2.0), 1.0) - 2.0).abs() < 1.0e-12);
        assert!((apart(&belt(1.0), 1.0) - 1.0).abs() < 1.0e-12);
    }
}
