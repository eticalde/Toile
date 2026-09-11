/// Area-weighted vertex normals: accumulate each triangle's `cross(p1-p0,
/// p2-p0)` into its three vertices, then normalize.
///
/// Duplicated from `toile_body`'s identical helper rather than shared: this
/// crate carries zero dependencies, including on its workspace siblings, so
/// the golden path never grows a chain of crates under it. The two are kept
/// in the same `+ - * / sqrt` regime on purpose — a platform's IEEE-754 sqrt
/// is correctly rounded, which is what makes both bit-identical on macOS ARM
/// and Linux x86.
///
/// The asset's CCW winding makes the result point outward; a vertex with no
/// face (none exist in the shipped body, but a malformed asset could have
/// one) falls back to the up axis rather than a zero vector.
pub(crate) fn vertex_normals(positions: &[f32], tris: &[u32]) -> Vec<f32> {
    let mut out = vec![0.0f32; positions.len()];
    let at = |i: u32| {
        let i = i as usize * 3;
        [positions[i], positions[i + 1], positions[i + 2]]
    };
    for t in tris.as_chunks::<3>().0 {
        let (pa, pb, pc) = (at(t[0]), at(t[1]), at(t[2]));
        let e1 = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
        let e2 = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        for &v in t {
            let v = v as usize * 3;
            out[v] += n[0];
            out[v + 1] += n[1];
            out[v + 2] += n[2];
        }
    }
    for nrm in out.as_chunks_mut::<3>().0 {
        let len = (nrm[0] * nrm[0] + nrm[1] * nrm[1] + nrm[2] * nrm[2]).sqrt();
        if len < 1.0e-12 {
            nrm[1] = 1.0;
        } else {
            nrm[0] /= len;
            nrm[1] /= len;
            nrm[2] /= len;
        }
    }
    out
}
