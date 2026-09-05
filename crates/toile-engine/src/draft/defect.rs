use thiserror::Error;
use toile_doc::formula::EvalError;
use toile_doc::{Axis, PointKey};
use toile_geom::validate;

/// What is wrong with a piece, in terms the drawing can point at.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Defect {
    /// A coordinate that does not resolve to a number.
    #[error("the {axis:?} of point {} does not resolve: {error}", point.index())]
    Binding {
        /// The point that carries it: a node of the contour or a handle.
        point: PointKey,
        /// Which of its two coordinates.
        axis: Axis,
        /// Why it did not resolve.
        error: EvalError,
    },
    /// A contour running through a point the document does not carry.
    #[error("the contour runs through point {}, which the document has lost", point.index())]
    NoSuchPoint {
        /// The point the contour still names.
        point: PointKey,
    },
    /// A contour that is not a simple closed polygon. Its indices are
    /// positions in the flattened contour, which for a piece of straight
    /// tracts are its nodes.
    #[error(transparent)]
    Contour(validate::ContourFault),
}
