//! The 2D pattern document: Toile's single source of truth.
//!
//! Every mutation goes through a reversible command, so undo is a property of
//! the model rather than a feature layered on top of it.

/// A place on a contour: a node plus a fraction of the tract leaving it.
mod anchor;
/// A store that never recycles an index.
mod arena;
/// What a coordinate is bound to.
mod binding;
/// The base blocks Toile brings, drafted over generic measurements.
pub mod block;
/// The reversible edits the document accepts.
mod command;
/// The wedge a dart takes out of a contour.
mod dart;
/// The document itself.
mod doc;
/// What can go wrong while reading or writing the document.
mod error;
/// The formula language a coordinate can be written in.
pub mod formula;
/// The stable identity of a document entity.
mod key;
/// The measurements a pattern resolves against.
mod measure;
/// A mark on a contour and the mark it answers to.
mod notch;
/// A pattern piece and its contour.
mod piece;
/// A point of cloth held to a place in space.
mod pin;
/// A control point of the pattern.
mod point;
/// Two stretches of contour sewn to each other.
mod seam;
/// An axis a piece is folded or mirrored on.
mod symmetry;
/// A quantity the pattern names once and reads everywhere.
mod variable;

pub use anchor::{EdgeAnchor, EdgeRange};
pub use arena::Arena;
pub use binding::Binding;
pub use command::{Applied, ChangeClass, Command};
pub use dart::{Dart, DartWedge, FoldDirection, WedgeNode};
pub use doc::Doc;
pub use error::DocError;
pub use key::{
    DartKey, Identity, Key, MannequinKey, NotchKey, PieceKey, PinKey, PointKey, SeamKey,
    SymmetryKey, VariableKey,
};
pub use measure::MeasureSet;
pub use notch::{Notch, NotchCount};
pub use piece::{ContourNode, Grain, Piece, Segment, Winding};
pub use pin::Pin;
pub use point::{Axis, Point};
pub use seam::{Seam, SeamKind, SeamOrientation};
pub use symmetry::{Symmetry, SymmetryKind};
pub use variable::Variable;
