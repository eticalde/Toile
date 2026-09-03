use super::state::State;

/// Per-vertex normals, as interleaved xyz into `out`.
///
/// Runs at visual cadence, not per substep: nothing in the solver reads them.
///
/// # Panics
/// If `out` is shorter than three floats per particle.
pub fn vertex_normals(state: &State, tris: &[u32], out: &mut [f32]) {
    out.fill(0.0);
    for t in tris.as_chunks::<3>().0 {
        let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
        let (e1x, e1y, e1z) = (
            state.px[b] - state.px[a],
            state.py[b] - state.py[a],
            state.pz[b] - state.pz[a],
        );
        let (e2x, e2y, e2z) = (
            state.px[c] - state.px[a],
            state.py[c] - state.py[a],
            state.pz[c] - state.pz[a],
        );
        let nx = e1y * e2z - e1z * e2y;
        let ny = e1z * e2x - e1x * e2z;
        let nz = e1x * e2y - e1y * e2x;
        for &v in &[a, b, c] {
            out[v * 3] += nx;
            out[v * 3 + 1] += ny;
            out[v * 3 + 2] += nz;
        }
    }
    for n in out.as_chunks_mut::<3>().0 {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1.0e-9);
        n[0] /= len;
        n[1] /= len;
        n[2] /= len;
    }
}
