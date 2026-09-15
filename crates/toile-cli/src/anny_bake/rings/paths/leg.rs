use super::super::Joints;
use super::super::arc::{Along, Section};
use super::super::geom::{cross, nearest_vertex, sub};
use super::super::intersect::Crossing;
use super::super::trunk::TrunkRings;
use super::{ACROSS, waist_side};

/// The four paths cut on the right leg's own section.
pub(super) struct SidePaths {
    pub inseam: Vec<Crossing>,
    pub outseam: Vec<Crossing>,
    pub rise: Vec<Crossing>,
    pub hip_drop: Vec<Crossing>,
}

/// `largo_lateral`, `tiro`, `altura_cadera` and `entrepierna`, all read off
/// one section of the right leg.
///
/// The plane holds the body's left-right axis, so it parts the leg into front
/// and back and its section runs down the outside of the leg on one side and
/// the inside on the other. It passes through two landmarks: the waist ring's
/// outermost point on the right, where the outer line therefore starts, and
/// the inner ankle bone — the vertex nearest the ankle joint, which on this
/// mesh is the medial malleolus — which the inner line therefore runs over.
/// On the template the outer line crosses the outer ankle bone's own bump at
/// that same height, and the inner line starts where the section's two legs
/// meet at the fork: the plane holds the left-right axis, so nothing of the
/// other leg lies between.
///
/// The outer line stops at the ankle bone's height for `largo_lateral`, the
/// fork's for `tiro` and the hip ring's for `altura_cadera`: three lengths
/// down the same side, each the start of the next.
pub(super) fn bake(
    positions: &[[f64; 3]],
    tris: &[[u32; 3]],
    j: &Joints,
    trunk: &TrunkRings,
) -> SidePaths {
    let waist = waist_side(positions, trunk);
    let bone = positions[nearest_vertex(positions, j.ankle) as usize];
    let leg = Section::through(
        positions,
        tris,
        waist,
        cross(sub(bone, waist), ACROSS),
        waist,
    );

    let top = leg.nearest(waist);
    let down = leg.downward(top);
    let side_to = |y: f64| leg.arc(top, leg.down_to(top, down, y), down);

    let ankle = leg.nearest(bone);
    let heel = leg.downward(ankle);
    let fork = leg.walk_until(ankle, heel.reversed(), |_, p| p[0] >= 0.0);
    let start = leg.next(fork, heel);

    SidePaths {
        inseam: leg.arc(start, heel_edge(&leg, ankle, heel), heel),
        outseam: side_to(bone[1]),
        rise: side_to(trunk.fork_y),
        hip_drop: side_to(trunk.hip_y),
    }
}

/// Where the inside of the heel turns under toward the sole: walking `down`
/// from the ankle bone at `from`, the last crossing before the first chord
/// that runs further across than it drops.
///
/// That is where a tape pressed down the inside of the leg leaves the foot.
/// Above it the side of the heel is steeper than it is wide, and the tape
/// lies on it; below it the section follows the sole, flat under the foot,
/// where no tape taken to the floor lies. The rest of the way down is
/// `toile_anny::asset::PathId::ends_on_floor`'s plumb drop, clear of a sole
/// that curves away beneath it.
fn heel_edge(leg: &Section, from: usize, down: Along) -> usize {
    let flat = leg.walk_until(from, down, |p, q| {
        let (dx, dz) = (q[0] - p[0], q[2] - p[2]);
        p[1] - q[1] < (dx * dx + dz * dz).sqrt()
    });
    leg.next(flat, down.reversed())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Down a wall, round a rounded corner, then along the floor: the edge is
    /// the last point before the first chord flatter than it is steep.
    #[test]
    fn the_heel_edge_is_where_the_side_turns_under() {
        let positions = vec![
            [0.0, 1.0, 0.0],
            [0.0, 0.5, 0.0],
            [0.1, 0.2, 0.0],
            [0.4, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
        ];
        let s = Section::new(&positions, (0..6).map(|i| (i, i, 0.0)).collect());
        assert_eq!(heel_edge(&s, 0, Along::Forward), 2);
    }
}
