use std::collections::BTreeMap;

use crate::{Error, Formula, FormulaError, Id, Measurements, Pattern};

/// Headings to directions and back: the only trigonometry in the crate.
pub(crate) mod angle;
/// Cubic Beziers and their arc length.
mod cubic;
/// The names a formula can cite.
pub(crate) mod env;
/// One construction object at a time, in file order.
mod walk;

pub use cubic::{Cubic, Quadrature};
use env::Env;

/// A position on the page in centimetres, y growing downward.
pub type Xy = [f64; 2];

/// Where the value of a `Spl_a_b` name comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplineLength {
    /// The curve's arc length, integrated: what the name means.
    ArcLength,
    /// The length the file wrote on the spline, which goes stale when the
    /// curve moves. It reproduces evaluators that trust that cache, so they
    /// can be compared point by point; it is not for importing.
    Written,
}

/// Every value a pattern defines, for one body.
#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    /// Every point, by id.
    pub points: BTreeMap<Id, Located>,
    /// The control points of every spline, by id.
    pub splines: BTreeMap<Id, Cubic>,
    /// The segments of every spline path, in path order, by id.
    pub paths: BTreeMap<Id, Vec<Cubic>>,
    /// Every arc, by id.
    pub arcs: BTreeMap<Id, CircleArc>,
    /// Where each cut point fell on its curve, by the point's id.
    pub cuts: BTreeMap<Id, Cut>,
    env: Env,
}

/// A point and its name.
#[derive(Debug, Clone, PartialEq)]
pub struct Located {
    /// The name the file gives it.
    pub name: String,
    /// Where it is.
    pub at: Xy,
}

/// An evaluated arc of a circle, from `angle1` counter-clockwise to `angle2`
/// as seen on the page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircleArc {
    /// The centre.
    pub center: Xy,
    /// The radius.
    pub radius: f64,
    /// Where it starts, in degrees.
    pub angle1: f64,
    /// Where it ends, in degrees.
    pub angle2: f64,
    /// The point at `angle1`.
    pub start: Xy,
    /// The point at `angle2`.
    pub end: Xy,
}

/// Where a cut point fell on the spline it cuts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cut {
    /// The spline cut.
    pub spline: Id,
    /// The arc length from the spline's start.
    pub length: f64,
    /// The curve parameter at that length.
    pub t: f64,
}

impl CircleArc {
    /// The angle the arc sweeps, in degrees, in [0, 360).
    pub fn sweep(&self) -> f64 {
        (self.angle2 - self.angle1).rem_euclid(360.0)
    }
}

impl Evaluation {
    /// Evaluates every variable and every construction object of `pattern`
    /// against `measurements`, in file order.
    ///
    /// # Errors
    ///
    /// A formula citing a name with no value at its point of the file, an
    /// arithmetic that leaves the finite numbers, or a construction with no
    /// answer: parallel lines, a line through a single point, a cut longer
    /// than its curve. Each names the element at fault.
    pub fn new(
        pattern: &Pattern,
        measurements: &Measurements,
        lengths: SplineLength,
    ) -> Result<Self, Error> {
        walk::evaluate(pattern, measurements, lengths)
    }

    /// The value of the pattern variable `name`, `#` included.
    pub fn variable(&self, name: &str) -> Option<f64> {
        self.env.variable(name)
    }

    /// The value a drawing gives `name`: `Line_a_b`, `AngleLine_a_b` or
    /// `Spl_a_b`.
    pub fn drawn(&self, name: &str) -> Option<f64> {
        self.env.drawn(name)
    }

    /// Evaluates `formula` with every name the whole pattern defines.
    ///
    /// # Errors
    ///
    /// A name the pattern does not define, or a non-finite result.
    pub fn eval(&self, formula: &Formula) -> Result<f64, FormulaError> {
        self.env.eval(formula, None)
    }

    /// The point called `name`.
    pub fn point_named(&self, name: &str) -> Option<Xy> {
        self.points
            .values()
            .find(|point| point.name == name)
            .map(|point| point.at)
    }
}
