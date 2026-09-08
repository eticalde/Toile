mod head;
pub(crate) mod limb;
pub(crate) mod trunk;

pub(crate) use limb::{arm, leg};
pub(crate) use trunk::{shoulder_half_width, trunk};

use crate::landmarks::Landmarks;
use crate::params::{BodyMeasures, BodyRes};
use crate::profile::nearest;
use crate::ring::{Ring, Shape, from_girth};
use crate::station::Station;

/// One lofted part: f32 xyz vertices, CCW triangle indices into them, and the
/// station tag of every vertex.
pub(crate) type Part = (Vec<f32>, Vec<u32>, Vec<u8>);

/// The station tag of every ring a tube through `table` emits at `steps`
/// rings per span.
pub(crate) fn tags(table: &[Station], steps: u32) -> Vec<u8> {
    nearest(table.len(), steps)
        .into_iter()
        .map(|i| table[i].tag())
        .collect()
}

/// Which side of the body a limb hangs on; left is −x.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Side {
    Left,
    Right,
}

impl Side {
    /// The sign of x on this side.
    pub(crate) fn sign(self) -> f64 {
        match self {
            Side::Left => -1.0,
            Side::Right => 1.0,
        }
    }
}

/// Everything a part builder reads, bundled so the builders stay under
/// clippy's argument cap and share one direction table.
pub(crate) struct Ctx<'a> {
    pub m: &'a BodyMeasures,
    pub res: BodyRes,
    pub lm: &'a Landmarks,
    pub dirs: &'a [(f64, f64)],
}

impl Ctx<'_> {
    /// A tape ring on the body axis (`cx = 0`) at `y`, carried `cz` forward.
    pub(crate) fn girth_ring(&self, girth_cm: f64, s: Shape, cz: f64, y: f64) -> Ring {
        from_girth(girth_cm, s, 0.0, cz, y, self.dirs)
    }
}

/// Moves a limb built in its local frame (joint at the origin, axis along −y)
/// onto the body: rotate by the pose angle `(c, s)` about z, outward on
/// `side`, then translate to `joint`. On both sides the map is a proper
/// rotation (`det = c² + s² = 1`), so the CCW-outward winding survives with
/// no mirroring and no index swap.
pub(crate) fn place(rings: &mut [Vec<[f64; 3]>], side: f64, joint: [f64; 3], c: f64, s: f64) {
    for p in rings.iter_mut().flatten() {
        let [x, y, z] = *p;
        *p = [
            joint[0] + x * c - side * y * s,
            joint[1] + side * x * s + y * c,
            joint[2] + z,
        ];
    }
}
