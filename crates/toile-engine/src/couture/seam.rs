use super::pipeline::ShapePipeline;

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
