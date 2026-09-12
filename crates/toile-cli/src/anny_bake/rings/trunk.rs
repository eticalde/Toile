use super::{Cut, Joints, UP, bands, intersect, select};

/// How far below `pecho`'s own found height `bajo_pecho` is cut.
const UNDERBUST_DROP_M: f64 = 0.04;

/// How far below the armpit apex — the highest horizontal plane that still
/// clears both arms, see [`bands::highest_unfused_trunk_y`] — `pecho_alto`
/// is cut.
///
/// The apex itself is where a *plane* stops meeting the arms, not where a
/// *tape* can lie, and the difference is measurable: cut exactly there, the
/// ring grazes the armpit crease and buckles out of its own plane by 4.20 cm
/// on the solved default tape, against 1.74 cm for `pecho` and 0.46 cm for
/// `cintura`. One centimetre lower it is already 2.20 cm and from there it
/// decays smoothly, so the anomaly is confined to the crease itself. Backing
/// off two centimetres — about the width of the tape that would be lying
/// there — puts it at 2.09 cm, in the band the other trunk rings occupy,
/// and still leaves it 4.7 cm clear of `pecho`'s own ring. This is the same
/// departure `legs::ANKLE_T` and `arms::WRIST_T` make, for the same reason:
/// the section exactly at the boundary is not a girth anyone could measure.
const UPPER_CHEST_DROP_M: f64 = 0.02;

/// The band scanned around the waist joint for `cintura`, in metres above
/// and below it. Asymmetric on purpose: this mesh's true narrowest point
/// sits well above the waist joint, toward the underbust, so the band
/// reaches further up than down to actually contain it.
const WAIST_BAND_BELOW_M: f64 = 0.02;
const WAIST_BAND_ABOVE_M: f64 = 0.11;
const WAIST_STEP_M: f64 = 0.005;

/// How far below the narrowest section — the natural indentation
/// [`bands::trunk_extremum`] finds — the `cintura` ring is actually cut.
///
/// That indentation is a rib-cage landmark on this mesh, not a lumbar one:
/// it lands 1.3 cm under `joint-spine-2`, the thoracolumbar joint, and a
/// full 8.0 cm above `joint-spine-3`, the lumbar joint whose height the band
/// is centred on. A tape tied there rides the bottom of the ribs. Every
/// length the catalogue takes from the waist — up to the nape, down to the
/// crotch and the hip — is instead a tailoring measurement, taken at the
/// waistline a garment sits on, which is lower.
///
/// How much lower is not something this mesh states, so the size of the
/// drop is settled by what the solved body does with it rather than by
/// counting scan steps: one template centimetre is the smallest drop that
/// brings `altura_cadera` inside tolerance of its dado. Going further buys
/// nothing — a larger drop keeps `altura_cadera` closed but widens the
/// waist-to-crotch rise as the crotch falls with it.
const WAIST_DROP_M: f64 = 0.010;

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
    pub upper_chest: Cut,
    pub bust: Cut,
    pub underbust: Cut,
    pub waist: Cut,
    pub hip: Cut,
    pub crotch: Cut,
    pub head: Cut,
}

/// A horizontal trunk ring at a height already known to be valid — used for
/// the rings this bake places directly rather than by scanning a band.
///
/// # Panics
/// If no valid trunk loop is found at `y`: a placement mistake, since every
/// caller passes a height this module already confirmed (or derived from
/// one that was confirmed).
fn ring_at(positions: &[[f64; 3]], tris: &[[u32; 3]], y: f64) -> Cut {
    let found = intersect::loops(positions, tris, [0.0, y, 0.0], UP);
    let points = select::pick_trunk_loop(positions, &found)
        .unwrap_or_else(|| panic!("no valid trunk cross-section at y = {y}"));
    Cut::along(points, UP)
}

pub(super) fn bake(
    positions: &[[f64; 3]],
    tris: &[[u32; 3]],
    j: &Joints,
    bust_apex_y: f64,
) -> TrunkRings {
    let upper_chest_y = bands::highest_unfused_trunk_y(
        positions,
        tris,
        f64::midpoint(j.scapula_r[1], j.clavicle_r[1]),
    ) - UPPER_CHEST_DROP_M;
    let upper_chest = ring_at(positions, tris, upper_chest_y);

    // No band, no search: `bust_apex_y` already answers where the apex is,
    // read off the breast targets themselves rather than guessed from a
    // joint — see `crate::anny_bake::bust_apex`.
    let bust = ring_at(positions, tris, bust_apex_y);
    let underbust = ring_at(positions, tris, bust_apex_y - UNDERBUST_DROP_M);

    let (_, indentation_y) = bands::trunk_extremum(
        positions,
        tris,
        j.spine3[1] - WAIST_BAND_BELOW_M,
        j.spine3[1] + WAIST_BAND_ABOVE_M,
        WAIST_STEP_M,
        false,
    );
    let waist = ring_at(positions, tris, indentation_y - WAIST_DROP_M);

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
