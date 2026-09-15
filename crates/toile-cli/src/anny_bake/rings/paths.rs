mod arm;
mod leg;

use toile_anny::asset::PathId;

use super::arc::Section;
use super::geom::{most_posterior_in_band, topmost_vertex_within};
use super::intersect::Crossing;
use super::trunk::TrunkRings;
use super::{Joints, select};

/// The body's own left-right axis, in the asset's frame.
const ACROSS: [f64; 3] = [1.0, 0.0, 0.0];

/// How far from a shoulder joint the shoulder point search looks: generous
/// enough to cover the deltoid cap over the joint, tight enough that it
/// cannot wander onto the chest or the neck.
const ACROMION_SEARCH_RADIUS_M: f64 = 0.07;

/// How far the nape search reaches from the sagittal midline: tight enough
/// that it cannot wander onto a shoulder blade, generous enough to find the
/// C7 vertebra's own bump even though it is not perfectly centred.
const NAPE_MIDLINE_TOLERANCE_M: f64 = 0.03;

/// The height band the nape search covers, as a margin above and below the
/// neck joint. Searching lower, down toward the shoulder joint, lands on the
/// upper back rather than the base of the neck.
const NAPE_BAND_MARGIN_M: f64 = 0.02;

/// The seven length paths, in no order of their own: [`Paths::in_id_order`]
/// lays them out for the asset.
pub(super) struct Paths {
    pub inseam: Vec<Crossing>,
    pub outseam: Vec<Crossing>,
    pub rise: Vec<Crossing>,
    pub hip_drop: Vec<Crossing>,
    pub back: Vec<Crossing>,
    pub arm: Vec<Crossing>,
    pub shoulders: Vec<Crossing>,
}

impl Paths {
    /// The paths in [`PathId::ALL`] order.
    pub fn in_id_order(self) -> [Vec<Crossing>; PathId::COUNT] {
        [
            self.inseam,
            self.outseam,
            self.rise,
            self.hip_drop,
            self.back,
            self.arm,
            self.shoulders,
        ]
    }
}

/// The top of the deltoid cap over the shoulder joint at `joint`: the
/// shoulder point, where the arm's length starts and the shoulders' width
/// ends.
fn shoulder_point(positions: &[[f64; 3]], joint: [f64; 3]) -> [f64; 3] {
    positions[topmost_vertex_within(positions, joint, ACROMION_SEARCH_RADIUS_M) as usize]
}

/// Cuts every length path from the neutral template.
///
/// `trunk` supplies the heights the side paths stop at and the waist ring the
/// side one starts from, so a length and the girth it runs from agree on
/// where that girth is.
pub(super) fn bake(
    positions: &[[f64; 3]],
    tris: &[[u32; 3]],
    j: &Joints,
    trunk: &TrunkRings,
) -> Paths {
    let side = leg::bake(positions, tris, j, trunk);
    let (arm, shoulders) = arm::bake(positions, tris, j);
    Paths {
        inseam: side.inseam,
        outseam: side.outseam,
        rise: side.rise,
        hip_drop: side.hip_drop,
        back: back(positions, tris, j, trunk.waist_y),
        arm,
        shoulders,
    }
}

/// `largo_espalda`: down the spine from the nape's height to the waist's.
///
/// Cut on `x = 0`, which is the template's mirror plane: the baker centres the
/// body's bounding box and the mesh is left-right symmetric, so the section
/// there runs down the middle of the back, over each vertebra. The nape
/// landmark is the most posterior vertex near the neck joint's height, and
/// sits a little beside the spine, on the muscle either side of it; only its
/// height is kept, and the path starts where the midline reaches that height.
fn back(positions: &[[f64; 3]], tris: &[[u32; 3]], j: &Joints, waist_y: f64) -> Vec<Crossing> {
    let nape = positions[most_posterior_in_band(
        positions,
        j.neck[1] - NAPE_BAND_MARGIN_M,
        j.neck[1] + NAPE_BAND_MARGIN_M,
        NAPE_MIDLINE_TOLERANCE_M,
    ) as usize];
    let on_spine = [0.0, nape[1], nape[2]];
    let spine = Section::through(positions, tris, on_spine, ACROSS, on_spine);
    let start = spine.nearest(on_spine);
    let down = spine.downward(start);
    spine.arc(start, spine.down_to(start, down, waist_y), down)
}

/// The side of the waist the outseam starts from: the waist ring's point
/// furthest toward the body's right.
fn waist_side(positions: &[[f64; 3]], trunk: &TrunkRings) -> [f64; 3] {
    let points = trunk
        .waist
        .points
        .iter()
        .map(|&c| select::ring_point(positions, c));
    points
        .min_by(|a, b| a[0].total_cmp(&b[0]))
        .expect("the waist ring is a loop, never empty")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_paths_come_out_in_the_assets_order() {
        let path = |v: u32| vec![(v, v, 0.0)];
        let paths = Paths {
            inseam: path(0),
            outseam: path(1),
            rise: path(2),
            hip_drop: path(3),
            back: path(4),
            arm: path(5),
            shoulders: path(6),
        };
        for (i, p) in paths.in_id_order().iter().enumerate() {
            let id = PathId::ALL[i];
            assert_eq!(p[0].0 as usize, id as usize, "{id:?} is out of place");
        }
    }
}
