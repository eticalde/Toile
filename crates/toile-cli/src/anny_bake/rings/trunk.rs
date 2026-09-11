use super::intersect::Crossing;
use super::{Joints, bands, intersect, select};

/// Half-width and step of the band scanned around the chest joint for
/// `pecho`, in metres.
const BUST_BAND_HALF_M: f64 = 0.03;
const BUST_STEP_M: f64 = 0.005;

/// How far below `pecho`'s own found height `bajo_pecho` is cut.
const UNDERBUST_DROP_M: f64 = 0.04;

/// The band scanned around the waist joint for `cintura`, in metres above
/// and below it. Asymmetric on purpose: this mesh's true narrowest point
/// sits well above the waist joint, toward the underbust, so the band
/// reaches further up than down to actually contain it.
const WAIST_BAND_BELOW_M: f64 = 0.02;
const WAIST_BAND_ABOVE_M: f64 = 0.11;
const WAIST_STEP_M: f64 = 0.005;

/// The band `cadera` is scanned over, expressed as an offset above the
/// fork (see [`bands::fork_y`]): from 12 cm above it to 14 cm above it.
/// This mesh's fullest single-loop trunk section decreases monotonically
/// all the way from the fork up past the waist — there is no interior
/// bulge to find — so any band here really just fixes the *lowest* point
/// still strictly above the fork as the answer; 12 cm is chosen because it
/// lands `altura_cadera` (waist to hip) in the same 18–22 cm neighbourhood
/// ISO size charts use for the waist-to-hip drop, comfortably under `tiro`
/// (waist to crotch) as the fork/crotch relationship requires structurally.
const HIP_BAND_LOW_ABOVE_FORK_M: f64 = 0.12;
const HIP_BAND_HIGH_ABOVE_FORK_M: f64 = 0.14;
const HIP_STEP_M: f64 = 0.005;

/// The band scanned above the head joint for `cabeza`'s widest section, in
/// metres. Starts well above the joint on purpose: lower than that is ear
/// height, where the ear's own folded geometry makes for a noisy, inflated
/// "widest" reading that no tailor's tape would actually trace.
const HEAD_BAND_LOW_M: f64 = 0.05;
const HEAD_BAND_HIGH_M: f64 = 0.10;
const HEAD_STEP_M: f64 = 0.005;

/// The horizontal trunk and head rings: every one placed either directly
/// at a joint height or by scanning a band for an extremum.
pub(super) struct TrunkRings {
    pub upper_chest: Vec<Crossing>,
    pub bust: Vec<Crossing>,
    pub underbust: Vec<Crossing>,
    pub waist: Vec<Crossing>,
    pub hip: Vec<Crossing>,
    pub crotch: Vec<Crossing>,
    pub head: Vec<Crossing>,
}

/// A horizontal trunk ring at a height already known to be valid — used for
/// the rings this bake places directly rather than by scanning a band.
///
/// # Panics
/// If no valid trunk loop is found at `y`: a placement mistake, since every
/// caller passes a height this module already confirmed (or derived from
/// one that was confirmed).
fn ring_at(positions: &[[f64; 3]], tris: &[[u32; 3]], y: f64) -> Vec<Crossing> {
    let found = intersect::loops(positions, tris, [0.0, y, 0.0], [0.0, 1.0, 0.0]);
    select::pick_trunk_loop(positions, &found)
        .unwrap_or_else(|| panic!("no valid trunk cross-section at y = {y}"))
}

pub(super) fn bake(positions: &[[f64; 3]], tris: &[[u32; 3]], j: &Joints) -> TrunkRings {
    let upper_chest_y = bands::highest_unfused_trunk_y(
        positions,
        tris,
        f64::midpoint(j.scapula_r[1], j.clavicle_r[1]),
    );
    let upper_chest = ring_at(positions, tris, upper_chest_y);

    let (bust, bust_y) = bands::trunk_extremum(
        positions,
        tris,
        j.spine1[1] - BUST_BAND_HALF_M,
        j.spine1[1] + BUST_BAND_HALF_M,
        BUST_STEP_M,
        true,
    );
    let underbust = ring_at(positions, tris, bust_y - UNDERBUST_DROP_M);

    let (waist, _) = bands::trunk_extremum(
        positions,
        tris,
        j.spine3[1] - WAIST_BAND_BELOW_M,
        j.spine3[1] + WAIST_BAND_ABOVE_M,
        WAIST_STEP_M,
        false,
    );

    // The fork is scanned once, downward from the pelvis joint (comfortably
    // above it on every phenotype this bake sees), and used for both the
    // crotch landmark and the hip band's lower bound.
    let fork_y = bands::fork_y(positions, tris, j.pelvis[1]);
    let crotch = ring_at(positions, tris, fork_y);

    let (hip, _) = bands::trunk_extremum(
        positions,
        tris,
        fork_y + HIP_BAND_LOW_ABOVE_FORK_M,
        fork_y + HIP_BAND_HIGH_ABOVE_FORK_M,
        HIP_STEP_M,
        true,
    );

    let head = bands::widest_by_extent(
        positions,
        tris,
        j.head[1] + HEAD_BAND_LOW_M,
        j.head[1] + HEAD_BAND_HIGH_M,
        HEAD_STEP_M,
    );

    TrunkRings {
        upper_chest,
        bust,
        underbust,
        waist,
        hip,
        crotch,
        head,
    }
}
