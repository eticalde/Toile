//! Reads a Seamly pattern, evaluates it, and translates it into a parametric
//! Toile product. Pure: it takes XML text, never a path. What it does not
//! model is an error naming the element, never a silent skip.

/// What can go wrong reading or evaluating a file, and where.
mod error;
/// Evaluating the construction graph against a body.
mod eval;
/// The formula language a Seamly attribute is written in.
mod formula;
/// The construction graph: points, lines, curves and arcs, in file order.
mod graph;
/// Seamly measurement names and the Toile catalogue names they stand for.
mod mapping;
/// A body's measurements, as a Seamly individual measurement file holds them.
mod measure;
/// The paths that walk the graph: internal lines and piece outlines.
mod path;
/// The pattern as a Toile product.
mod product;
/// Formulas as linear forms, for unrolling a construction exactly.
mod sym;
/// Reading attributes so that none goes unread.
mod xml;

pub use error::{Error, Place};
pub use eval::{CircleArc, Cubic, Cut, Evaluation, Located, Quadrature, SplineLength, Xy};
pub use formula::{Expr, Formula, FormulaError, Op};
pub use graph::{
    Arc, Block, Comment, Construction, Id, Object, ObjectKind, PathPoint, Pattern, Spline, Variable,
};
pub use mapping::{measurement_pairs, seamly_measurement, toile_measurement};
pub use measure::{Measurement, Measurements};
pub use path::{InternalPath, NodeKind, Notch, PathNode, Piece};
pub use product::{
    Carried, Curve, End, FrozenNote, HelperKind, HelperNote, InternalNote, LengthNote, Measure,
    NotchNote, PieceNote, Product, Refusal, Report, Source, VariableNote, import,
};
pub use sym::Frozen;
