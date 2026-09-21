use crate::Xy;

// Gauss–Legendre with 8 nodes on [-1, 1], positive half: exact for a
// polynomial up to degree 15, so a spline with both handles at zero length,
// whose speed is a quadratic, integrates exactly on the first panel.
const NODES: [f64; 4] = [
    0.183_434_642_495_649_8,
    0.525_532_409_916_329,
    0.796_666_477_413_626_7,
    0.960_289_856_497_536_3,
];
const WEIGHTS: [f64; 4] = [
    0.362_683_783_378_362,
    0.313_706_645_877_887_27,
    0.222_381_034_453_374_48,
    0.101_228_536_290_376_26,
];

/// A panel is accepted once halving it moves its integral by less than
/// this, in centimetres. Halving shrinks an 8-node rule's error by about
/// 2^16, so the accepted halves are far closer than the step that accepted
/// them; the step is still what [`Quadrature::error`] adds up.
const PANEL_TOLERANCE: f64 = 1e-12;
/// Past this depth a panel is accepted whatever its step, which only a
/// curve with a cusp could reach.
const MAX_DEPTH: u32 = 30;
/// How close a cut's arc length must land on the length asked for, in
/// centimetres.
const CUT_TOLERANCE: f64 = 1e-11;
/// Newton steps a cut may take; each failed step bisects instead, so this
/// many always suffice.
const CUT_STEPS: usize = 64;

/// A cubic Bezier by its four control points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cubic(pub [Xy; 4]);

/// An integral and an upper estimate of its error.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quadrature {
    /// The integral.
    pub value: f64,
    /// The sum, over accepted panels, of how much halving the panel moved
    /// it: an estimate of the error before the halving, so an overestimate
    /// of the error of `value`.
    pub error: f64,
}

impl Cubic {
    /// The point at parameter `t`.
    pub fn point(&self, t: f64) -> Xy {
        let [p0, p1, p2, p3] = self.0;
        let u = 1.0 - t;
        let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        [
            a * p0[0] + b * p1[0] + c * p2[0] + d * p3[0],
            a * p0[1] + b * p1[1] + c * p2[1] + d * p3[1],
        ]
    }

    /// The derivative at parameter `t`.
    pub fn velocity(&self, t: f64) -> Xy {
        let [p0, p1, p2, p3] = self.0;
        let u = 1.0 - t;
        let (a, b, c) = (3.0 * u * u, 6.0 * u * t, 3.0 * t * t);
        [
            a * (p1[0] - p0[0]) + b * (p2[0] - p1[0]) + c * (p3[0] - p2[0]),
            a * (p1[1] - p0[1]) + b * (p2[1] - p1[1]) + c * (p3[1] - p2[1]),
        ]
    }

    fn speed(&self, t: f64) -> f64 {
        let [x, y] = self.velocity(t);
        (x * x + y * y).sqrt()
    }

    /// The curve's arc length.
    pub fn length(&self) -> f64 {
        self.arc_length(1.0).value
    }

    /// The arc length from the start to parameter `t`, by adaptive
    /// Gauss–Legendre quadrature of the speed.
    pub fn arc_length(&self, t: f64) -> Quadrature {
        if t.is_nan() || t <= 0.0 {
            return Quadrature {
                value: 0.0,
                error: 0.0,
            };
        }
        let t = t.min(1.0);
        self.adaptive(0.0, t, self.gauss(0.0, t), 0)
    }

    /// The parameter at arc length `length` from the start, or `None` when
    /// the curve is shorter.
    pub fn t_at_length(&self, length: f64) -> Option<f64> {
        let total = self.length();
        if !(0.0..=total + CUT_TOLERANCE).contains(&length) {
            return None;
        }
        if length >= total {
            return Some(1.0);
        }
        let (mut lo, mut hi) = (0.0, 1.0);
        let mut t = length / total;
        for _ in 0..CUT_STEPS {
            let miss = self.arc_length(t).value - length;
            if miss.abs() <= CUT_TOLERANCE {
                break;
            }
            if miss > 0.0 {
                hi = t;
            } else {
                lo = t;
            }
            let newton = t - miss / self.speed(t);
            t = if newton > lo && newton < hi {
                newton
            } else {
                f64::midpoint(lo, hi)
            };
        }
        Some(t)
    }

    fn gauss(&self, a: f64, b: f64) -> f64 {
        let half = 0.5 * (b - a);
        let mid = f64::midpoint(a, b);
        let mut sum = 0.0;
        for (node, weight) in NODES.iter().zip(WEIGHTS) {
            let offset = half * node;
            sum += weight * (self.speed(mid - offset) + self.speed(mid + offset));
        }
        sum * half
    }

    fn adaptive(&self, a: f64, b: f64, whole: f64, depth: u32) -> Quadrature {
        let mid = f64::midpoint(a, b);
        let (left, right) = (self.gauss(a, mid), self.gauss(mid, b));
        let step = (left + right - whole).abs();
        if step <= PANEL_TOLERANCE || depth == MAX_DEPTH {
            return Quadrature {
                value: left + right,
                error: step,
            };
        }
        let left = self.adaptive(a, mid, left, depth + 1);
        let right = self.adaptive(mid, b, right, depth + 1);
        Quadrature {
            value: left.value + right.value,
            error: left.error + right.error,
        }
    }
}
