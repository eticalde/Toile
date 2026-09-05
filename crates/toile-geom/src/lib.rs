//! Pure, deterministic 2D geometry.
//!
//! Every function is total and side-effect free: same inputs, same bits, on
//! every platform. The drape goldens rest on that.

/// Deterministic evaluation and flattening of a cubic Bezier.
pub mod curve;
/// The arc length of a contour and of its runs.
pub mod length;
/// Sampling a contour by arc length.
pub mod sample;
/// Checking that a contour is a simple closed polygon.
pub mod validate;
