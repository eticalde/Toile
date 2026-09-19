use super::pipeline::ShapePipeline;

/// Seam compliance a product is let go with.
///
/// Soft, because a product is let go with its seams open: the pieces are
/// placed around the body and the cloth between two of them has yet to be
/// pulled together. A firm seam over that gap is a detonation, not a drape.
pub const SEAM_SOFT: f32 = 1.0e-5;

/// Seam compliance once the product has been pulled together: firm enough
/// that a sewn pair reads as one point.
pub const SEAM_FIRM: f32 = 1.0e-9;

/// Most a sewn pair may be pulled together in one pass, in metres.
///
/// This and not the compliance is what governs a wide gap: the correction a
/// pair asks for is half the distance between its two ends, and over anything
/// but a closed seam that is far more than the cap allows.
pub const SEAM_STEP: f32 = 0.002;

/// The same cap once the seams are firm, when what is left to close is the
/// residue rather than the gap.
const SEAM_STEP_FIRM: f32 = 0.01;

/// Seam passes per substep. Seams are few and dominate the conditioning, so
/// iterating them is cheap and closes what one capped pass cannot.
pub const SEAM_PASSES: u32 = 4;

/// Substeps the compliance takes to go from soft to firm.
const RAMP: u64 = 450;

/// How near a sewn pair has to stand for the seam to count as shut, in metres.
///
/// A tenth of the spacing the mesher lays boundary vertices at: inside that,
/// the two sides are one line as far as the cloth is concerned, and what is
/// left is the residue a firm seam holds rather than a gap.
const SHUT: f32 = 0.0009;

/// Whether the product is still being pulled together `substeps` into a
/// drape, with its widest sewn pair `gap` metres apart.
///
/// This is the one question gravity waits on. A garment is let go with its
/// seams open — 8 cm apart on the shipped block — and letting it fall while it
/// is still in pieces drags the halves past each other before they can meet;
/// `GarmentCodeData` closes the seams first for exactly that reason.
///
/// The gap itself and not a counted number of substeps, because it measures
/// the thing being waited for: a two-piece tube and a twelve-piece jacket are
/// shut at different moments, and a count tuned on one would let the other
/// fall half-made. Both are deterministic; only one is about the garment.
///
/// The ramp is the cap, and less a second criterion than the end of the
/// first: past it the sewing is firm, so a pair still open has had the pulling
/// it is going to get and will not close by hanging any longer. A garment
/// whose seams cannot meet falls late rather than never.
pub fn closing(substeps: u64, gap: f32) -> bool {
    substeps < RAMP && gap > SHUT
}

/// How stiff the seams are, and how far they may pull, `substeps` into a
/// drape: the schedule the seams benchmark settled on.
pub fn sewing_at(substeps: u64) -> (f32, f32) {
    if substeps >= RAMP {
        return (SEAM_FIRM, SEAM_STEP_FIRM);
    }
    let t = substeps as f32 / RAMP as f32;
    // `libm` rather than the intrinsic, for the reason the bake pins it: this
    // runs on every substep of a sewn drape, and std's `powf` is the
    // platform's, so the same garment would be pulled together a little
    // differently on the second architecture. The contract here allows
    // `+ - * / sqrt` and this one pinned crate, and names `powf` among what
    // it does not.
    (SEAM_SOFT * libm::powf(SEAM_FIRM / SEAM_SOFT, t), SEAM_STEP)
}

/// Pairs two boundary runs for sewing: `count` pairs at equal relative
/// fractions of each run.
///
/// A run is where it starts and how far it goes, never its two ends: a
/// caller holding a span and handing over `(start, start + span)` cannot get
/// it back, the round trip losing it for about a quarter of a contour's
/// pairs. An ulp, and no vertex has been seen to flip on it — but determinism
/// here is meant to hold by construction.
///
/// A negative span runs the side backwards, and one carrying the start past 1
/// or under 0 passes the closure: `(0.9, 0.3)` runs from nine tenths round to
/// two tenths. Ease comes from a mismatch in the two lengths, not a gather
/// parameter; each side is offset by its own, so both halves come back in the
/// solver's one space. Fewer come back where a boundary vertex repeats.
///
/// # Panics
/// If `count` is less than two: a seam needs both endpoints.
pub fn pair_seam(
    a: &ShapePipeline,
    run_a: (f64, f64),
    a_offset: u32,
    b: &ShapePipeline,
    run_b: (f64, f64),
    b_offset: u32,
    count: usize,
) -> (Vec<u32>, Vec<u32>) {
    assert!(count >= 2, "a seam needs at least two pairs, got {count}");
    // Each side states its own base, so a seam between the second and third
    // pieces can be expressed at all: a side pinned to zero is only correct
    // where its piece opens the combined state, and anywhere else it names
    // another piece's vertices — in bounds, in silence, closing the garment
    // on a seam that is not there.
    let mut va = Vec::with_capacity(count);
    let mut vb = Vec::with_capacity(count);
    for k in 0..count {
        let t = k as f64 / (count - 1) as f64;
        let fa = run_a.0 + run_a.1 * t;
        let fb = run_b.0 + run_b.1 * t;
        let (pa, pb) = (
            a.boundary_vertex_near(fa) + a_offset,
            b.boundary_vertex_near(fb) + b_offset,
        );
        if va.last() == Some(&pa) || vb.last() == Some(&pb) {
            continue;
        }
        va.push(pa);
        vb.push(pb);
    }
    (va, vb)
}
