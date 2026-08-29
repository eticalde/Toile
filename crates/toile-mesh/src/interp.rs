//! Interpolación del interior desde la frontera — Spike 2 (issue #34).
//!
//! Mean Value Coordinates (Floater 2003) como primera implementación del
//! trait de interpolación. El ADR §2.2 pide PMVC (Lipman 2007) para piezas
//! cóncavas: MVC puede dar pesos negativos en sisas/escotes profundos — el
//! Spike 5 decide con la suite de piezas cóncavas si el upgrade es
//! necesario. La matriz se precomputa una vez; cada edición de forma es
//! solo la multiplicación (matriz densa interior × frontera).
//!
//! Nota de determinismo: usa transcendentales de std (sqrt solamente en el
//! camino de apply; atan no aparece). Los goldens cross-plataforma de la
//! matriz se evalúan en este spike.

/// Pesos densos: `weights[j * n_boundary + i]` = influencia del vértice de
/// frontera `i` sobre el vértice interior `j`.
pub struct BoundaryInterp {
    pub n_boundary: usize,
    pub weights: Vec<f64>,
}

/// Precomputa los pesos MVC de cada punto interior respecto al polígono de
/// frontera (cerrado, sin duplicar el punto inicial).
pub fn mvc_weights(boundary: &[[f64; 2]], interior: &[[f64; 2]]) -> BoundaryInterp {
    let n = boundary.len();
    let mut weights = vec![0.0f64; interior.len() * n];

    for (j, p) in interior.iter().enumerate() {
        let row = &mut weights[j * n..(j + 1) * n];
        // Distancias y tan(α_i/2) por arista (v_i, v_{i+1}).
        let d: Vec<f64> = boundary
            .iter()
            .map(|v| ((v[0] - p[0]).powi(2) + (v[1] - p[1]).powi(2)).sqrt())
            .collect();

        // Punto pegado a un vértice de frontera: peso delta.
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

/// Aplica la matriz: interior nuevo = pesos × frontera nueva.
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
