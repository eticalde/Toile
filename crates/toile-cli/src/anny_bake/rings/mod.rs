mod arms;
mod bands;
mod geom;
mod intersect;
mod legs;
mod select;
mod trunk;

use geom::joint;
use intersect::Crossing;
use toile_anny::asset::{RingId, RingPoint, RingRange};

/// Every named joint this bake reads from `groups::joint_centroids`'s
/// output, gathered once so the rest of this module reads names instead of
/// repeating lookups.
struct Joints {
    neck: [f64; 3],
    head: [f64; 3],
    upper_leg: [f64; 3],
    knee: [f64; 3],
    ankle: [f64; 3],
    shoulder_r: [f64; 3],
    shoulder_l: [f64; 3],
    elbow_r: [f64; 3],
    elbow_l: [f64; 3],
    hand_r: [f64; 3],
    spine3: [f64; 3],
    pelvis: [f64; 3],
    scapula_r: [f64; 3],
    clavicle_r: [f64; 3],
}

fn gather(joints: &[(String, [f64; 3])]) -> Joints {
    Joints {
        neck: joint(joints, "joint-neck"),
        head: joint(joints, "joint-head"),
        upper_leg: joint(joints, "joint-r-upper-leg"),
        knee: joint(joints, "joint-r-knee"),
        ankle: joint(joints, "joint-r-ankle"),
        shoulder_r: joint(joints, "joint-r-shoulder"),
        shoulder_l: joint(joints, "joint-l-shoulder"),
        elbow_r: joint(joints, "joint-r-elbow"),
        elbow_l: joint(joints, "joint-l-elbow"),
        hand_r: joint(joints, "joint-r-hand"),
        spine3: joint(joints, "joint-spine-3"),
        pelvis: joint(joints, "joint-pelvis"),
        scapula_r: joint(joints, "joint-r-scapula"),
        clavicle_r: joint(joints, "joint-r-clavicle"),
    }
}

/// Cuts a ring perpendicular to `axis` through `point`, one limb's own
/// cross-section among its siblings — see [`select::pick_limb_loop`].
/// Shared by [`legs`] and [`arms`], the two submodules whose rings are all
/// built this same way.
///
/// # Panics
/// If the plane there does not separate this limb from its sibling
/// structure: a real placement mistake in the caller, not a data problem.
fn limb_ring(
    positions: &[[f64; 3]],
    tris: &[[u32; 3]],
    point: [f64; 3],
    axis: [f64; 3],
    label: &str,
) -> Vec<Crossing> {
    let found = intersect::loops(positions, tris, point, axis);
    select::pick_limb_loop(positions, &found, point)
        .unwrap_or_else(|| panic!("{label}: no valid, unfused limb cross-section at {point:?}"))
}

/// Bakes all nineteen rings from the neutral template, in [`RingId::ALL`]
/// order, ready to flatten into [`toile_anny::asset::Baked`]'s ring table.
///
/// `positions` and `joints` must already be in the mesh's own final space
/// (centred, scaled to metres) — the same space `positions` itself is
/// stored in. See `crate::anny_bake`'s doc on the two placements that
/// depart from their catalogue name's literal joint.
pub fn bake(
    positions: &[[f64; 3]],
    tris: &[[u32; 3]],
    joints: &[(String, [f64; 3])],
    bust_apex_y: f64,
) -> (Vec<RingRange>, Vec<RingPoint>) {
    let j = gather(joints);
    let legs = legs::bake(positions, tris, &j);
    let arms = arms::bake(positions, tris, &j);
    let trunk = trunk::bake(positions, tris, &j, bust_apex_y);

    let rings = [
        legs.neck,
        trunk.upper_chest,
        trunk.bust,
        trunk.underbust,
        trunk.waist,
        trunk.hip,
        legs.thigh,
        legs.knee,
        legs.ankle,
        arms.upper_arm,
        arms.wrist,
        trunk.head,
        trunk.crotch,
        arms.shoulder_r,
        arms.shoulder_l,
        arms.elbow,
        arms.acromion,
        arms.wrist_joint,
        legs.nape,
    ];
    debug_assert_eq!(rings.len(), RingId::COUNT);
    flatten(rings)
}

/// Quantizes nineteen crossing-loops into the asset's flat `(RingRange,
/// RingPoint)` shape, in the array's own order (which is [`RingId::ALL`]).
///
/// # Panics
/// If a body vertex index does not fit `u16`: it never should, since the
/// body group is 13,380 vertices and every crossing names two of them.
fn flatten(rings: [Vec<Crossing>; RingId::COUNT]) -> (Vec<RingRange>, Vec<RingPoint>) {
    let mut ranges = Vec::with_capacity(RingId::COUNT);
    let mut points = Vec::new();
    for ring in rings {
        let offset = points.len() as u32;
        for (a, b, t) in ring {
            points.push(RingPoint {
                vertex_a: u16::try_from(a).expect("body vertex index fits u16"),
                vertex_b: u16::try_from(b).expect("body vertex index fits u16"),
                t: t as f32,
            });
        }
        ranges.push(RingRange {
            offset,
            length: (points.len() as u32) - offset,
        });
    }
    (ranges, points)
}

#[cfg(test)]
mod tests;
