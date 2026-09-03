//! The 2D pattern document: Toile's single source of truth.
//!
//! Every mutation goes through a reversible command, so undo is a property of
//! the model rather than a feature layered on top of it.

/// Document entities and the commands that mutate them.
pub mod model;
