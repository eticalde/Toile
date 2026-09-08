use crate::ring::{Ring, point};

/// The ring fields the profile curves: everything but `y`, which stays linear
/// inside a span so rings never fold and every station sits at its landmark.
const FIELDS: usize = 7;

fn fields(r: &Ring) -> [f64; FIELDS] {
    [
        r.a,
        r.depth_front,
        r.depth_back,
        r.round_front,
        r.round_back,
        r.cx,
        r.cz,
    ]
}

fn ring_from(y: f64, f: [f64; FIELDS]) -> Ring {
    Ring {
        cx: f[5],
        cz: f[6],
        y,
        a: f[0],
        depth_front: f[1],
        depth_back: f[2],
        round_front: f[3],
        round_back: f[4],
    }
}

/// Every ring of a profile through ordered stations: `steps` interpolated
/// rings inside each span, each station emitted exactly once and bit-exact.
///
/// The shape fields follow a monotone cubic Hermite (PCHIP) through the
/// stations, so a girth never overshoots its neighbours and a roundness stays
/// in [0, 1]; only `+ - * /` are used, which keeps the loft inside the
/// determinism contract.
pub(crate) fn interpolate(secs: &[Ring], steps: u32) -> Vec<Ring> {
    let ys: Vec<f64> = secs.iter().map(|r| r.y).collect();
    let cols: Vec<Vec<f64>> = (0..FIELDS)
        .map(|j| secs.iter().map(|r| fields(r)[j]).collect())
        .collect();
    let tans: Vec<Vec<f64>> = cols.iter().map(|c| tangents(&ys, c)).collect();
    let denom = f64::from(steps + 1);
    let mut out = Vec::new();
    for i in 0..secs.len().saturating_sub(1) {
        let h = ys[i + 1] - ys[i];
        for k in 0..=steps {
            let s = f64::from(k) / denom;
            let mut f = [0.0; FIELDS];
            for (j, v) in f.iter_mut().enumerate() {
                *v = hermite(cols[j][i], tans[j][i], cols[j][i + 1], tans[j][i + 1], h, s);
            }
            out.push(ring_from(ys[i] + h * s, f));
        }
    }
    if let Some(last) = secs.last() {
        out.push(*last);
    }
    out
}

/// The point rings of a tube through ordered stations.
pub(crate) fn tube(secs: &[Ring], steps: u32, dirs: &[(f64, f64)]) -> Vec<Vec<[f64; 3]>> {
    interpolate(secs, steps)
        .iter()
        .map(|r| points(r, dirs))
        .collect()
}

/// A quarter-ellipse dome from `base` (not re-emitted) to an apex at `apex_y`,
/// whose last ring collapses onto the apex. The centre and roundness are
/// carried up unchanged, so a head keeps its forward lean to the crown.
pub(crate) fn dome(
    base: &Ring,
    apex_y: f64,
    rings: u32,
    dirs: &[(f64, f64)],
) -> Vec<Vec<[f64; 3]>> {
    let n = f64::from(rings);
    (1..=rings)
        .map(|i| {
            let s = f64::from(i) / n;
            let k = (1.0 - s * s).sqrt();
            let r = Ring {
                a: base.a * k,
                depth_front: base.depth_front * k,
                depth_back: base.depth_back * k,
                y: base.y + (apex_y - base.y) * s,
                ..*base
            };
            points(&r, dirs)
        })
        .collect()
}

fn points(r: &Ring, dirs: &[(f64, f64)]) -> Vec<[f64; 3]> {
    dirs.iter().map(|&d| point(r, d)).collect()
}

/// PCHIP tangents with Fritsch–Butland weights, which handle the unequal
/// spans between landmarks. An extremum (the waist minimum, the shoulder
/// maximum) gets a zero tangent so the curve never overshoots; the ends are
/// flat, which meets the skull dome C1 and keeps the hidden caps flat.
fn tangents(ys: &[f64], vs: &[f64]) -> Vec<f64> {
    let n = ys.len();
    let mut d = vec![0.0; n];
    for i in 1..n.saturating_sub(1) {
        let (h0, h1) = (ys[i] - ys[i - 1], ys[i + 1] - ys[i]);
        let (s0, s1) = ((vs[i] - vs[i - 1]) / h0, (vs[i + 1] - vs[i]) / h1);
        // Divide only when both slopes are non-zero and agree in sign.
        if s0 * s1 > 0.0 {
            let (w1, w2) = (2.0 * h1 + h0, h1 + 2.0 * h0);
            d[i] = (w1 + w2) / (w1 / s0 + w2 / s1);
        }
    }
    d
}

/// The cubic Hermite basis on a span of length `h`, evaluated in this fixed
/// order (the golden pins it). At `s = 0` the basis is (1, 0, 0, 0), so a
/// station reproduces its value bit-exactly.
fn hermite(v0: f64, d0: f64, v1: f64, d1: f64, h: f64, s: f64) -> f64 {
    let s2 = s * s;
    let s3 = s2 * s;
    (2.0 * s3 - 3.0 * s2 + 1.0) * v0
        + (s3 - 2.0 * s2 + s) * h * d0
        + (-2.0 * s3 + 3.0 * s2) * v1
        + (s3 - s2) * h * d1
}
