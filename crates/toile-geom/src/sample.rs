//! Muestreo de contornos por fracciones de longitud de arco.
//!
//! La clave del drapeado incremental (ADR §2.2): las MISMAS fracciones
//! sobre un contorno editado producen la misma cantidad de vértices de
//! frontera en el mismo orden → la conectividad de la malla no cambia.

/// `n` fracciones uniformes en [0, 1).
pub fn uniform_fractions(n: usize) -> Vec<f64> {
    (0..n).map(|i| i as f64 / n as f64).collect()
}

/// Muestrea un polígono cerrado en las fracciones dadas de su perímetro.
pub fn sample_closed(contour: &[[f64; 2]], fractions: &[f64]) -> Vec<[f64; 2]> {
    let n = contour.len();
    let mut cum = Vec::with_capacity(n + 1);
    let mut total = 0.0;
    cum.push(0.0);
    for i in 0..n {
        let (a, b) = (contour[i], contour[(i + 1) % n]);
        total += ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        cum.push(total);
    }

    let mut out = Vec::with_capacity(fractions.len());
    let mut seg = 0usize;
    for &f in fractions {
        let target = f * total;
        // Las fracciones llegan ordenadas: avance monótono del segmento.
        while seg + 1 < n && cum[seg + 1] < target {
            seg += 1;
        }
        let (a, b) = (contour[seg], contour[(seg + 1) % n]);
        let len = cum[seg + 1] - cum[seg];
        let t = if len > 0.0 {
            (target - cum[seg]) / len
        } else {
            0.0
        };
        out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
    }
    out
}
