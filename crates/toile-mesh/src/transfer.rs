/// A uniform bin grid over a 2D mesh, for locating points inside it.
///
/// This is how a re-meshed piece inherits the live drape: each vertex of the
/// new mesh is located in the *rest space* of the old one and its 3D state
/// interpolated barycentrically, so the simulation continues instead of
/// restarting.
///
/// Location is deterministic by construction: bins are scanned in canonical
/// ring order, the first containing triangle wins, and when numerical error
/// leaves a point just outside every candidate, the one with the largest
/// minimum barycentric wins — ties going to the lower triangle index.
pub struct Locator<'a> {
    verts: &'a [[f64; 2]],
    tris: &'a [u32],
    min: [f64; 2],
    cell: [f64; 2],
    nx: usize,
    ny: usize,
    bins: Vec<Vec<u32>>,
}

impl<'a> Locator<'a> {
    /// Bins every triangle of the mesh by its bounding box.
    pub fn build(verts: &'a [[f64; 2]], tris: &'a [u32]) -> Self {
        let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
        for v in verts {
            for k in 0..2 {
                lo[k] = lo[k].min(v[k]);
                hi[k] = hi[k].max(v[k]);
            }
        }
        let (nx, ny) = (64usize, 64usize);
        let cell = [
            ((hi[0] - lo[0]) / nx as f64).max(1.0e-12),
            ((hi[1] - lo[1]) / ny as f64).max(1.0e-12),
        ];
        let mut bins = vec![Vec::new(); nx * ny];
        for (t, tri) in tris.chunks(3).enumerate() {
            let (mut tlo, mut thi) = ([f64::MAX; 2], [f64::MIN; 2]);
            for &v in tri {
                let p = verts[v as usize];
                for k in 0..2 {
                    tlo[k] = tlo[k].min(p[k]);
                    thi[k] = thi[k].max(p[k]);
                }
            }
            let (i0, i1) = (
                cell_of(tlo[0], lo[0], cell[0], nx),
                cell_of(thi[0], lo[0], cell[0], nx),
            );
            let (j0, j1) = (
                cell_of(tlo[1], lo[1], cell[1], ny),
                cell_of(thi[1], lo[1], cell[1], ny),
            );
            for j in j0..=j1 {
                for i in i0..=i1 {
                    bins[j * nx + i].push(t as u32);
                }
            }
        }
        Self {
            verts,
            tris,
            min: lo,
            cell,
            nx,
            ny,
            bins,
        }
    }

    /// Returns the triangle containing `p` and its barycentric coordinates,
    /// falling back to the nearest candidate, clamped, when none contains it.
    pub fn locate(&self, p: [f64; 2]) -> (usize, [f64; 3]) {
        let ci = cell_of(p[0], self.min[0], self.cell[0], self.nx) as isize;
        let cj = cell_of(p[1], self.min[1], self.cell[1], self.ny) as isize;
        let mut best: (f64, usize, [f64; 3]) = (f64::MIN, 0, [1.0, 0.0, 0.0]);
        for ring in 0..(self.nx.max(self.ny) as isize) {
            for j in (cj - ring).max(0)..=(cj + ring).min(self.ny as isize - 1) {
                for i in (ci - ring).max(0)..=(ci + ring).min(self.nx as isize - 1) {
                    // Only the new ring; the interior was scanned already.
                    if ring > 0 && (i - ci).abs() != ring && (j - cj).abs() != ring {
                        continue;
                    }
                    for &t in &self.bins[j as usize * self.nx + i as usize] {
                        let b = self.bary(t as usize, p);
                        let worst = b[0].min(b[1]).min(b[2]);
                        if worst >= -1.0e-9 {
                            return (t as usize, clamp_bary(b));
                        }
                        if worst > best.0 {
                            best = (worst, t as usize, b);
                        }
                    }
                }
            }
            // One extra ring past the first candidate, then settle: a point
            // outside the mesh would otherwise scan the whole grid.
            if ring >= 2 && best.0 > f64::MIN {
                break;
            }
        }
        (best.1, clamp_bary(best.2))
    }

    fn bary(&self, t: usize, p: [f64; 2]) -> [f64; 3] {
        let (a, b, c) = (
            self.verts[self.tris[t * 3] as usize],
            self.verts[self.tris[t * 3 + 1] as usize],
            self.verts[self.tris[t * 3 + 2] as usize],
        );
        let det = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
        if det.abs() < 1.0e-18 {
            return [1.0, 0.0, 0.0];
        }
        let w1 = ((p[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (p[1] - a[1])) / det;
        let w2 = ((b[0] - a[0]) * (p[1] - a[1]) - (p[0] - a[0]) * (b[1] - a[1])) / det;
        [1.0 - w1 - w2, w1, w2]
    }
}

fn cell_of(x: f64, min: f64, cell: f64, n: usize) -> usize {
    (((x - min) / cell) as usize).min(n - 1)
}

fn clamp_bary(b: [f64; 3]) -> [f64; 3] {
    let c = [b[0].max(0.0), b[1].max(0.0), b[2].max(0.0)];
    let s = c[0] + c[1] + c[2];
    [c[0] / s, c[1] / s, c[2] / s]
}

/// Counts triangles whose orientation flipped relative to a reference — the
/// fold-over detector for the interior interpolator.
pub fn count_flipped(reference: &[[f64; 2]], current: &[[f64; 2]], tris: &[u32]) -> usize {
    let area2 = |v: &[[f64; 2]], a: usize, b: usize, c: usize| {
        (v[b][0] - v[a][0]) * (v[c][1] - v[a][1]) - (v[c][0] - v[a][0]) * (v[b][1] - v[a][1])
    };
    tris.chunks(3)
        .filter(|t| {
            let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
            area2(reference, a, b, c) * area2(current, a, b, c) < 0.0
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit square split into two triangles.
    fn quad() -> (Vec<[f64; 2]>, Vec<u32>) {
        (
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            vec![0, 1, 2, 0, 2, 3],
        )
    }

    #[test]
    fn a_vertex_locates_with_a_unit_barycentric() {
        let (v, t) = quad();
        let loc = Locator::build(&v, &t);
        let (_, b) = loc.locate([0.0, 0.0]);
        assert!((b.iter().sum::<f64>() - 1.0).abs() < 1.0e-12);
        assert!(b.iter().any(|&x| (x - 1.0).abs() < 1.0e-9));
    }

    #[test]
    fn an_interior_point_reconstructs_from_its_barycentric() {
        let (v, t) = quad();
        let loc = Locator::build(&v, &t);
        let p = [0.7, 0.3];
        let (tri, b) = loc.locate(p);
        let (a, c, d) = (
            v[t[tri * 3] as usize],
            v[t[tri * 3 + 1] as usize],
            v[t[tri * 3 + 2] as usize],
        );
        let x = b[0] * a[0] + b[1] * c[0] + b[2] * d[0];
        let y = b[0] * a[1] + b[1] * c[1] + b[2] * d[1];
        assert!((x - p[0]).abs() < 1.0e-9 && (y - p[1]).abs() < 1.0e-9);
    }

    #[test]
    fn a_point_outside_still_yields_a_valid_barycentric() {
        let (v, t) = quad();
        let loc = Locator::build(&v, &t);
        let (_, b) = loc.locate([5.0, -3.0]);
        assert!((b.iter().sum::<f64>() - 1.0).abs() < 1.0e-12);
        assert!(b.iter().all(|&x| x >= 0.0));
    }

    #[test]
    fn an_unchanged_mesh_has_no_flips() {
        let (v, t) = quad();
        assert_eq!(count_flipped(&v, &v, &t), 0);
    }

    #[test]
    fn a_mirrored_mesh_flips_every_triangle() {
        let (v, t) = quad();
        let mirrored: Vec<[f64; 2]> = v.iter().map(|p| [-p[0], p[1]]).collect();
        assert_eq!(count_flipped(&v, &mirrored, &t), t.len() / 3);
    }
}
