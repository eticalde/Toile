#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_engine::couture::pair_seam;
use toile_engine::demo;

/// Panel width, in metres: the rectangle of the seams benchmark.
const W: f64 = 0.46;

/// Front panel height, in metres.
const H_FRONT: f64 = 0.55;

/// Back panel height, in metres: ten per cent shorter, as in the benchmark.
const H_BACK: f64 = 0.50;

/// Spacing of the contour nodes, in metres.
const STEP: f64 = 0.01;

/// How much the hem grows, in metres: the benchmark's hot edit.
const HEM_GROWTH: f64 = 0.06;

/// Pairs along the side seam, as the benchmark sews it.
const PAIRS: usize = 60;

/// A CCW rectangle sampled every `STEP`, with the arc fractions of its four
/// corners. The first corner is the hem end of the right-hand side seam.
fn rect_contour(w: f64, h: f64) -> (Vec<[f64; 2]>, [f64; 4]) {
    let per = 2.0 * (w + h);
    let mut pts = Vec::new();
    let mut line = |a: [f64; 2], b: [f64; 2]| {
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        let n = (len / STEP).ceil() as usize;
        for i in 0..n {
            let t = i as f64 / n as f64;
            pts.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        }
    };
    line([0.0, 0.0], [w, 0.0]);
    line([w, 0.0], [w, h]);
    line([w, h], [0.0, h]);
    line([0.0, h], [0.0, 0.0]);
    (pts, [w / per, (w + h) / per, (2.0 * w + h) / per, 1.0])
}

/// Widens the hem by `HEM_GROWTH`, symmetrically about the panel's centre.
///
/// Node count and node order are untouched, so this is a shape edit: the same
/// one the seams benchmark runs with the garment already on the avatar.
fn widen_hem(contour: &[[f64; 2]]) -> Vec<[f64; 2]> {
    contour
        .iter()
        .map(|p| {
            if p[1] < 1.0e-9 {
                [W * 0.5 + (p[0] - W * 0.5) * (1.0 + HEM_GROWTH / W), p[1]]
            } else {
                *p
            }
        })
        .collect()
}

/// The node and local fraction of the polygon point nearest `q`.
///
/// This is the material address of a point on the cloth: a node key survives
/// an edit, a fraction of the whole perimeter does not.
fn locate(polygon: &[[f64; 2]], q: [f64; 2]) -> (usize, f64) {
    let n = polygon.len();
    let mut best = (0usize, 0.0f64, f64::INFINITY);
    for i in 0..n {
        let (a, b) = (polygon[i], polygon[(i + 1) % n]);
        let (ex, ey) = (b[0] - a[0], b[1] - a[1]);
        let len2 = ex * ex + ey * ey;
        let t = if len2 > 0.0 {
            (((q[0] - a[0]) * ex + (q[1] - a[1]) * ey) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (dx, dy) = (a[0] + ex * t - q[0], a[1] + ey * t - q[1]);
        let d = dx * dx + dy * dy;
        if d < best.2 {
            best = (i, t, d);
        }
    }
    (best.0, best.1)
}

/// Where a material address sits on a polygon.
fn resolve(polygon: &[[f64; 2]], at: (usize, f64)) -> [f64; 2] {
    let (a, b) = (polygon[at.0], polygon[(at.0 + 1) % polygon.len()]);
    [a[0] + (b[0] - a[0]) * at.1, a[1] + (b[1] - a[1]) * at.1]
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt()
}

/// What the test measures, in metres.
struct Drift {
    /// How far the sewn vertex ends up from the cloth point it was sewn at.
    metres: f64,
    /// Mean arc distance between boundary vertices: the finest a pairing can
    /// resolve, since it must land on one of them.
    spacing: f64,
}

/// Sews the right-hand side seam of two panels, widens the front's hem, and
/// measures how far the seam's hem end slid over the cloth.
fn side_seam_head_drift() -> Drift {
    let (front_contour, ff) = rect_contour(W, H_FRONT);
    let (back_contour, fb) = rect_contour(W, H_BACK);
    let mut front = demo::pipeline(&front_contour);
    let back = demo::pipeline(&back_contour);
    let offset = front.pos2d.len() as u32;

    let (a, _) = pair_seam(&front, (ff[0], ff[1]), &back, (fb[0], fb[1]), offset, PAIRS);
    let head = a[0] as usize;
    let sewn_at = locate(&front_contour, front.pos2d[head]);

    let edited = widen_hem(&front_contour);
    front
        .derive(&edited)
        .expect("widening the hem moves nodes, it does not add any");

    Drift {
        metres: distance(front.pos2d[head], resolve(&edited, sewn_at)),
        spacing: perimeter(&edited) / front.n_boundary() as f64,
    }
}

fn perimeter(polygon: &[[f64; 2]]) -> f64 {
    (0..polygon.len())
        .map(|i| distance(polygon[i], polygon[(i + 1) % polygon.len()]))
        .sum()
}

/// The defect, measured. A seam is stored as a fraction of the whole
/// perimeter, so an edit that lengthens the perimeter slides every seam along
/// the cloth: the pair sewn at the hem corner ends up 36 mm away from it, for
/// as long as the anchoring stays global.
///
/// The band is wide on purpose. Its floor is the claim — the drift is real and
/// coarse, not rounding — and its ceiling catches a change that makes it worse
/// without anybody looking.
#[test]
fn global_fraction_anchoring_drifts_36_mm() {
    let drift = side_seam_head_drift();
    let mm = drift.metres * 1000.0;
    assert!(
        (30.0..45.0).contains(&mm),
        "the seam head drifted {mm:.1} mm; the defect this test documents is 36 mm"
    );
    assert!(
        drift.metres > drift.spacing * 2.0,
        "a drift of {mm:.1} mm has to exceed the mesh spacing to be the anchoring and not rounding"
    );
}
