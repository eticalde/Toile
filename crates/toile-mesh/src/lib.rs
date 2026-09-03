//! Per-piece meshing and interior reprojection.
//!
//! The same contour always triangulates to the same mesh, bit for bit. That is
//! a requirement, not an optimisation: the goldens compare across machines.

/// Constrained Delaunay triangulation and refinement.
pub mod cdt;
/// Reprojecting a piece's interior from its boundary.
pub mod interp;
/// Locating points in a mesh, to carry state onto a new one.
pub mod transfer;
