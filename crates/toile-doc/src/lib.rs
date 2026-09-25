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
/// A stretch of contour held to a ratio of the length it was drawn at.
mod elastic;
/// What can go wrong while reading or writing the document.
mod error;
/// The formula language a coordinate can be written in.
pub mod formula;
/// The document as canonical JSON.
mod json;
/// The stable identity of a document entity.
mod key;
/// A run of places a piece is drawn with and not cut on.
mod line;
/// The measurements a pattern resolves against.
mod measure;
/// A mark on a contour and the mark it answers to.
mod notch;
/// A person in the user's library, and the link a copied body keeps to them.
mod persona;
/// A pattern piece and its contour.
mod piece;
/// A point of cloth held to a place in space.
mod pin;
/// Where a piece sits on the product overview.
mod placement;
/// A control point of the pattern.
mod point;
/// Two stretches of contour sewn to each other.
mod seam;
/// What runs from one contour node to the next.
mod segment;
/// An axis a piece is folded or mirrored on.
mod symmetry;
/// A quantity the pattern names once and reads everywhere.
mod variable;

pub use anchor::{EdgeAnchor, EdgeRange};
pub use arena::Arena;
pub use binding::Binding;
pub use command::{Applied, ChangeClass, Coalesced, Command, History};
pub use dart::{Dart, DartWedge, FoldDirection, WedgeNode};
pub use doc::Doc;
pub use elastic::Elastic;
pub use error::DocError;
pub use json::{
    FormatError, PERSONA_EXTENSION, PERSONA_VERSION, VERSION as FORMAT_VERSION,
    VERSION_ELASTIC as FORMAT_VERSION_ELASTIC, VERSION_EXTENDED as FORMAT_VERSION_EXTENDED,
    VERSION_FOLDED as FORMAT_VERSION_FOLDED, VERSION_INTERNAL as FORMAT_VERSION_INTERNAL,
    VERSION_LINKED as FORMAT_VERSION_LINKED, VERSION_PLACED as FORMAT_VERSION_PLACED,
};
pub use key::{
    DartKey, ElasticKey, Identity, Key, LineKey, MannequinKey, NotchKey, PieceKey, PinKey,
    PointKey, SeamKey, SymmetryKey, VariableKey,
};
pub use line::{InternalLine, LineEdit, LineKind, LineSpan, LineVertex, SpanEdit, VertexEdit};
pub use measure::{BodyShape, MeasureSet};
pub use notch::{Notch, NotchCount};
pub use persona::{Origin, Persona, PersonaError, Snapshot};
pub use piece::{ContourNode, Grain, Piece, SAMPLES, Winding};
pub use pin::Pin;
pub use placement::Placement;
pub use point::{Axis, Point};
pub use seam::{Seam, SeamKind, SeamOrientation};
pub use segment::{Handle, Handles, Segment, SegmentEdit, Side};
pub use symmetry::{Symmetry, SymmetryKind};
pub use variable::Variable;
