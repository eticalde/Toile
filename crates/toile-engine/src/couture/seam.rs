use super::pipeline::ShapePipeline;

/// Pairs two boundary runs for sewing: `count` pairs at equal relative
/// fractions of each run.
///
/// When the two runs differ in length, ease emerges on its own from the
/// mismatch — no explicit gather parameter. Pass a range backwards, as
/// `(f1, f0)`, to reverse that side's direction. Indices from `b` are offset
/// by `b_offset`, since seams address the combined solver state.
///
/// Returns fewer than `count` pairs where the nearest boundary vertex repeats,
/// which happens on runs sampled more finely than the mesh.
///
/// # Panics
/// If `count` is less than two: a seam needs both endpoints.
pub fn pair_seam(
    a: &ShapePipeline,
    range_a: (f64, f64),
    b: &ShapePipeline,
    range_b: (f64, f64),
    b_offset: u32,
    count: usize,
) -> (Vec<u32>, Vec<u32>) {
    assert!(count >= 2, "a seam needs at least two pairs, got {count}");
    let mut va = Vec::with_capacity(count);
    let mut vb = Vec::with_capacity(count);
    for k in 0..count {
        let t = k as f64 / (count - 1) as f64;
        let fa = range_a.0 + (range_a.1 - range_a.0) * t;
        let fb = range_b.0 + (range_b.1 - range_b.0) * t;
        let (pa, pb) = (
            a.boundary_vertex_near(fa),
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
