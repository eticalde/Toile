/// Dense boundary-to-interior weights.
///
/// `weights[j * n_boundary + i]` is the influence of boundary vertex `i` on
/// interior vertex `j`.
#[allow(
    missing_docs,
    reason = "documented by the struct and the indexing note above"
)]
#[derive(Debug, Clone)]
pub struct BoundaryInterp {
    pub n_boundary: usize,
    pub weights: Vec<f64>,
}

/// Precomputes mean value coordinates (Floater 2003) for each interior point
/// against a closed boundary polygon.
///
/// Weights are computed once; a shape edit is then just the matrix product in
/// [`apply`], which is what keeps an edit under the interactive budget.
///
/// On strongly concave pieces — deep armholes, necklines — MVC can produce
/// negative weights and fold triangles over. PMVC (Lipman 2007) is the
/// documented replacement; the fold-over suite in `toile-cli` measures when it
/// becomes necessary.
///
/// The boundary must be closed and must not repeat its first point.
pub fn mvc_weights(boundary: &[[f64; 2]], interior: &[[f64; 2]]) -> BoundaryInterp {
    let n = boundary.len();
    let mut weights = vec![0.0f64; interior.len() * n];

    for (j, p) in interior.iter().enumerate() {
        let row = &mut weights[j * n..(j + 1) * n];
        let d: Vec<f64> = boundary
            .iter()
            .map(|v| ((v[0] - p[0]).powi(2) + (v[1] - p[1]).powi(2)).sqrt())
            .collect();

        // Sitting on a boundary vertex: the general formula divides by zero.
        if let Some(k) = d.iter().position(|&x| x < 1.0e-12) {
            row[k] = 1.0;
            continue;
        }

        let mut tan_half = vec![0.0f64; n];
        for i in 0..n {
            let ni = (i + 1) % n;
            let (ax, ay) = (boundary[i][0] - p[0], boundary[i][1] - p[1]);
            let (bx, by) = (boundary[ni][0] - p[0], boundary[ni][1] - p[1]);
            let cross = ax * by - ay * bx;
            let dot = ax * bx + ay * by;
            tan_half[i] = if cross.abs() < 1.0e-14 {
                0.0
            } else {
                (d[i] * d[ni] - dot) / cross
            };
        }
        let mut sum = 0.0;
        for i in 0..n {
            let prev = (i + n - 1) % n;
            let w = (tan_half[prev] + tan_half[i]) / d[i];
            row[i] = w;
            sum += w;
        }
        for w in row.iter_mut() {
            *w /= sum;
        }
    }
    BoundaryInterp {
        n_boundary: n,
        weights,
    }
}

/// Reprojects the interior from a new boundary: `out = weights × boundary`.
///
/// # Panics
/// If `boundary` is shorter than `interp.n_boundary`, or `out` is shorter than
/// the number of interior points the weights were built for.
pub fn apply(interp: &BoundaryInterp, boundary: &[[f64; 2]], out: &mut [[f64; 2]]) {
    let n = interp.n_boundary;
    for (j, o) in out.iter_mut().enumerate() {
        let row = &interp.weights[j * n..(j + 1) * n];
        let (mut x, mut y) = (0.0f64, 0.0f64);
        for (w, v) in row.iter().zip(boundary) {
            x += w * v[0];
            y += w * v[1];
        }
        *o = [x, y];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<[f64; 2]> {
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
    }

    #[test]
    fn every_row_is_a_partition_of_unity() {
        let interior = vec![[0.5, 0.5], [0.25, 0.75], [0.9, 0.1]];
        let w = mvc_weights(&square(), &interior);
        for row in w.weights.chunks(w.n_boundary) {
            assert!((row.iter().sum::<f64>() - 1.0).abs() < 1.0e-12);
        }
    }

    #[test]
    fn a_point_on_a_boundary_vertex_gets_a_delta_weight() {
        let w = mvc_weights(&square(), &[[1.0, 0.0]]);
        assert_eq!(w.weights, vec![0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn an_unchanged_boundary_reproduces_the_interior() {
        let b = square();
        let interior = vec![[0.5, 0.5], [0.25, 0.75], [0.9, 0.1]];
        let w = mvc_weights(&b, &interior);
        let mut out = vec![[0.0; 2]; interior.len()];
        apply(&w, &b, &mut out);
        for (got, want) in out.iter().zip(&interior) {
            assert!((got[0] - want[0]).abs() < 1.0e-12, "{got:?} vs {want:?}");
            assert!((got[1] - want[1]).abs() < 1.0e-12, "{got:?} vs {want:?}");
        }
    }

    #[test]
    fn a_uniform_scale_of_the_boundary_scales_the_interior() {
        let b = square();
        let w = mvc_weights(&b, &[[0.5, 0.5]]);
        let scaled: Vec<[f64; 2]> = b.iter().map(|p| [p[0] * 2.0, p[1] * 2.0]).collect();
        let mut out = [[0.0; 2]];
        apply(&w, &scaled, &mut out);
        assert!((out[0][0] - 1.0).abs() < 1.0e-12);
        assert!((out[0][1] - 1.0).abs() < 1.0e-12);
    }
}
