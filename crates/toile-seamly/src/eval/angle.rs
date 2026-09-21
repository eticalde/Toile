use crate::Xy;

/// The point `length` from `origin`, heading `degrees`: 0 east,
/// counter-clockwise as seen on a page whose y axis grows downward.
pub(crate) fn toward(origin: Xy, length: f64, degrees: f64) -> Xy {
    let [x, y] = heading(degrees);
    [origin[0] + length * x, origin[1] + length * y]
}

/// The heading from `from` to `to`, in degrees, in [0, 360].
pub(crate) fn bearing(from: Xy, to: Xy) -> f64 {
    libm::atan2(-(to[1] - from[1]), to[0] - from[0])
        .to_degrees()
        .rem_euclid(360.0)
}

/// How far a cubic's handles stand from the ends of the circular arc it
/// stands for, as a fraction of the radius, for an arc sweeping `degrees`.
///
/// `4/3 · tan(sweep / 4)` puts the cubic's midpoint on the circle and its
/// ends tangent to it, which is the usual cubic stand-in for an arc.
pub(crate) fn arc_handle_ratio(degrees: f64) -> f64 {
    4.0 / 3.0 * libm::tan(degrees.to_radians() / 4.0)
}

/// The unit vector heading `degrees`: 0 east, counter-clockwise as seen on a
/// page whose y axis grows downward.
#[allow(
    clippy::float_cmp,
    reason = "a quadrant heading is recognised exactly or not at all"
)]
pub(crate) fn heading(degrees: f64) -> Xy {
    // Drafting squares off far more than it slants, and an exact quadrant
    // keeps such a point on its base's axis to the bit: its translated
    // coordinate is the base's own, with no trigonometric residue.
    let turn = degrees.rem_euclid(360.0);
    if turn == 0.0 {
        [1.0, 0.0]
    } else if turn == 90.0 {
        [0.0, -1.0]
    } else if turn == 180.0 {
        [-1.0, 0.0]
    } else if turn == 270.0 {
        [0.0, 1.0]
    } else {
        let radians = degrees.to_radians();
        [libm::cos(radians), -libm::sin(radians)]
    }
}
