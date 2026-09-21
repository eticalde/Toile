use toile_doc::SAMPLES;

use crate::{Cubic, Xy};

/// The most a flattened curve may stray from the curve, in centimetres.
///
/// A tenth of a millimetre, the tolerance the shipped trouser block flattens
/// its own curves to: what a drag and the precision box round to, and a
/// tenth of what a pencil draws.
const TOLERANCE: f64 = 0.01;

/// Points probed on the curve between two samples, to find how far the chord
/// joining them strays.
const PROBES: u16 = 16;

/// How many samples a cubic needs, and how far its flattening then strays.
///
/// Toile flattens a tract at fixed fractions of the curve's parameter, so the
/// count is found the same way: the fewest samples, within the counts a tract
/// may ask for, whose chords stay within the tolerance of the curve.
pub(crate) fn samples(control: [Xy; 4]) -> (u16, f64) {
    let cubic = Cubic(control);
    let mut last = (SAMPLES.1, f64::INFINITY);
    for count in SAMPLES.0..=SAMPLES.1 {
        let off = stray(&cubic, count);
        last = (count, off);
        if off <= TOLERANCE {
            break;
        }
    }
    last
}

/// How far the curve strays from the chords of its flattening at `count`.
fn stray(cubic: &Cubic, count: u16) -> f64 {
    let n = f64::from(count);
    let mut worst: f64 = 0.0;
    for chord in 0..count {
        let i = f64::from(chord);
        let (a, b) = (cubic.point(i / n), cubic.point((i + 1.0) / n));
        for probe in 1..PROBES {
            let t = (i + f64::from(probe) / f64::from(PROBES)) / n;
            worst = worst.max(off_segment(cubic.point(t), a, b));
        }
    }
    worst
}

/// The distance from `p` to the segment from `a` to `b`.
fn off_segment(p: Xy, a: Xy, b: Xy) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let length2 = dx * dx + dy * dy;
    let along = if length2 > 0.0 {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / length2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (ex, ey) = (a[0] + along * dx - p[0], a[1] + along * dy - p[1]);
    (ex * ex + ey * ey).sqrt()
}

/// The points a closed contour flattens to, the way Toile flattens it: each
/// tract its own first point, then a curve's samples short of the next.
pub(crate) fn flattened(tracts: &[([Xy; 4], Option<u16>)]) -> Vec<Xy> {
    let mut out = Vec::new();
    for (control, samples) in tracts {
        out.push(control[0]);
        if let Some(count) = samples {
            let cubic = Cubic(*control);
            for step in 1..*count {
                out.push(cubic.point(f64::from(step) / f64::from(*count)));
            }
        }
    }
    out
}

/// Twice the signed area a closed polygon encloses, positive for one that
/// runs clockwise on a page whose y axis grows downward.
pub(crate) fn signed_area(polygon: &[Xy]) -> f64 {
    let mut sum = 0.0;
    for (index, a) in polygon.iter().enumerate() {
        let b = polygon[(index + 1) % polygon.len()];
        sum += a[0] * b[1] - b[0] * a[1];
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_straight_cubic_takes_the_fewest_samples_a_curve_may() {
        let (count, off) = samples([[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [3.0, 0.0]]);
        assert_eq!(count, SAMPLES.0);
        assert!(off < 1e-12);
    }

    #[test]
    fn a_quarter_circle_of_ten_centimetres_needs_more_than_a_few() {
        let k = 10.0 * 0.552_284_749_830_793_6;
        let (count, off) = samples([[10.0, 0.0], [10.0, k], [k, 10.0], [0.0, 10.0]]);
        assert!(off <= TOLERANCE, "{off}");
        assert!(count > 8 && count < SAMPLES.1, "{count}");
        assert!(
            stray(
                &Cubic([[10.0, 0.0], [10.0, k], [k, 10.0], [0.0, 10.0]]),
                count - 1
            ) > TOLERANCE
        );
    }

    #[test]
    fn a_clockwise_square_on_the_page_has_a_positive_area() {
        let square = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        assert!(signed_area(&square) > 0.0);
    }
}
