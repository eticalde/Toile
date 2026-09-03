//! Pure, deterministic 2D geometry.
//!
//! Every function is total and side-effect free: same inputs, same bits, on
//! every platform. The drape goldens rest on that.

/// Sampling a contour by arc length.
pub mod sample;
