use crate::parts::Part;

/// Grid-triangulates consecutive rings (each `seg + 1` wide, seam duplicated)
/// into CCW-outward quads, optionally fan-capping the first and last ring.
/// Each ring's vertices take that ring's station tag, and a cap's centre takes
/// its ring's. This is the one place the f64 geometry is cast to the
/// renderer's f32, after any placement, so a rotated arm is rounded once.
pub(crate) fn loft(rings: &[Vec<[f64; 3]>], tags: &[u8], cap_lo: bool, cap_hi: bool) -> Part {
    debug_assert_eq!(rings.len(), tags.len(), "one tag per ring");
    let width = rings.first().map_or(0, Vec::len);
    let seg = width.saturating_sub(1) as u32;
    let mut verts: Vec<f32> = Vec::new();
    let mut stations: Vec<u8> = Vec::new();
    for (ring, &tag) in rings.iter().zip(tags) {
        for p in ring {
            verts.extend(p.iter().map(|&c| c as f32));
        }
        stations.extend(std::iter::repeat_n(tag, ring.len()));
    }
    let mut idx: Vec<u32> = Vec::new();
    let width_u = width as u32;
    for r in 0..rings.len().saturating_sub(1) as u32 {
        let (lo, hi) = (r * width_u, (r + 1) * width_u);
        for j in 0..seg {
            let (a, b) = (lo + j, hi + j);
            // Rings run bottom→top, columns +x→+z: this winding faces outward.
            idx.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    if cap_lo && !rings.is_empty() {
        fan(&mut verts, &mut idx, 0, seg, true);
        stations.push(tags[0]);
    }
    if cap_hi && rings.len() > 1 {
        let start = (rings.len() - 1) as u32 * width_u;
        fan(&mut verts, &mut idx, start, seg, false);
        stations.push(tags[rings.len() - 1]);
    }
    (verts, idx, stations)
}

/// Fans the ring whose first vertex is `start` to a fresh centroid vertex.
/// `lo` caps face −y, `hi` caps face +y.
fn fan(verts: &mut Vec<f32>, idx: &mut Vec<u32>, start: u32, seg: u32, lo: bool) {
    let (mut cx, mut cy, mut cz) = (0.0f32, 0.0f32, 0.0f32);
    let first = start as usize * 3;
    for p in verts[first..first + seg as usize * 3].as_chunks::<3>().0 {
        cx += p[0];
        cy += p[1];
        cz += p[2];
    }
    let inv = 1.0 / seg as f32;
    let center = (verts.len() / 3) as u32;
    verts.extend_from_slice(&[cx * inv, cy * inv, cz * inv]);
    for j in 0..seg {
        let (a, b) = (start + j, start + j + 1);
        if lo {
            idx.extend_from_slice(&[center, b, a]);
        } else {
            idx.extend_from_slice(&[center, a, b]);
        }
    }
}

/// Area-weighted vertex normals: accumulate each triangle's `cross(p1-p0,
/// p2-p0)` into its three vertices, then normalize. Consistent CCW winding
/// makes them point outward; a degenerate vertex with no face falls back to up.
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
            // A collapsed apex or cap-centre vertex has no face normal; give it
            // the up axis so every normal the renderer reads is unit length.
            nrm[1] = 1.0;
        } else {
            nrm[0] /= len;
            nrm[1] /= len;
            nrm[2] /= len;
        }
    }
    out
}
