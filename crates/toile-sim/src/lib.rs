//! Resident XPBD cloth solver.
//!
//! The solver is never reset. Shape edits arrive as new rest values and are
//! swapped in between substeps, so an edit re-drapes instead of restarting.

/// The solver itself: state, constraints, collision, stepping.
pub mod xpbd;
