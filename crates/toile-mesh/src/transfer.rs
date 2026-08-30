//! Localización de puntos y transferencia baricéntrica — Spike 5 (#37).
//!
//! Vía B (cambio de topología): la malla nueva hereda el estado 3D vivo de
//! la vieja localizando cada vértice nuevo en el espacio 2D de reposo de la
//! malla vieja e interpolando baricéntricamente. Determinista: grilla de
//! bins en orden canónico, primer triángulo contenedor gana, fallback al de
//! mejor baricéntrica mínima (empates por índice menor).

/// Grilla de aceleración sobre los triángulos de una malla 2D.
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

    /// Triángulo que contiene a `p` y sus coordenadas baricéntricas; si
    /// ningún candidato lo contiene (borde numérico), el de mejor
    /// baricéntrica mínima entre los inspeccionados, con clamp.
    pub fn locate(&self, p: [f64; 2]) -> (usize, [f64; 3]) {
        let ci = cell_of(p[0], self.min[0], self.cell[0], self.nx) as isize;
        let cj = cell_of(p[1], self.min[1], self.cell[1], self.ny) as isize;
        let mut best: (f64, usize, [f64; 3]) = (f64::MIN, 0, [1.0, 0.0, 0.0]);
        for ring in 0..(self.nx.max(self.ny) as isize) {
            for j in (cj - ring).max(0)..=(cj + ring).min(self.ny as isize - 1) {
                for i in (ci - ring).max(0)..=(ci + ring).min(self.nx as isize - 1) {
                    // Solo el anillo nuevo (borde), no el interior repetido.
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
            // Contenedor no encontrado en este anillo: si ya hay un candidato
            // razonable tras revisar un anillo extra, aceptarlo.
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

/// Triángulos con orientación invertida respecto a la de referencia — el
/// detector de foldovers del interpolador (§3.4).
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
