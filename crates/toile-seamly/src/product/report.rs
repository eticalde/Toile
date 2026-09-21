use toile_doc::Winding;

use crate::{Frozen, Xy};

/// Everything the import did that a person should know: what was renamed,
/// what was added, what was frozen and what the product has nowhere to keep.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    /// The name the product's body was given.
    pub body: String,
    /// The measurements carried under a catalogue name.
    pub mapped: Vec<Measure>,
    /// The measurements a formula reads that the catalogue has no name for,
    /// carried under their own.
    pub carried: Vec<Measure>,
    /// The measurements left out of the body: no catalogue name, and no
    /// formula reads them.
    pub left_out: Vec<Measure>,
    /// The pattern's variables, in file order.
    pub variables: Vec<VariableNote>,
    /// The variables the translation added, in the order it needed them.
    pub helpers: Vec<HelperNote>,
    /// The quantities no formula can follow, frozen at the imported body.
    pub frozen: Vec<FrozenNote>,
    /// The along-line points whose direction was read off the imported body.
    pub directions: Vec<String>,
    /// Every spline the file writes a length on, in file order.
    pub lengths: Vec<LengthNote>,
    /// The pieces, in file order.
    pub pieces: Vec<PieceNote>,
    /// How many points the file constructs, and how many the product needed
    /// as a corner, a handle or a variable.
    pub construction: (usize, usize),
}

/// A measurement of the imported body.
#[derive(Debug, Clone, PartialEq)]
pub struct Measure {
    /// Its name in the measurement file.
    pub seamly: String,
    /// Its name in the product, if it has one there.
    pub toile: Option<String>,
    /// Centimetres.
    pub value: f64,
}

/// A variable of the pattern, as the product carries it.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableNote {
    /// Its name in the pattern, `#` included.
    pub seamly: String,
    /// Its name in the product.
    pub toile: String,
    /// Its formula in the product.
    pub formula: String,
    /// What its author wrote about it, which the product has no field for.
    pub description: String,
}

/// A variable the translation added.
#[derive(Debug, Clone, PartialEq)]
pub struct HelperNote {
    /// Its name.
    pub name: String,
    /// Its formula.
    pub formula: String,
    /// What it stands for in the pattern.
    pub stands_for: HelperKind,
}

/// What an added variable stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelperKind {
    /// A `Line_` or `Spl_` name a formula of the pattern cites.
    Drawn(String),
    /// A coordinate of the construction point of this name, which needs a
    /// square root and which other constructions build on.
    Coordinate(String),
}

/// A quantity frozen at the imported body.
#[derive(Debug, Clone, PartialEq)]
pub struct FrozenNote {
    /// What was frozen.
    pub frozen: Frozen,
    /// What the pattern calls it: the `Spl_` name, or the cut point's name.
    pub name: String,
    /// The number it was frozen at.
    pub value: f64,
    /// The points of the product whose place depends on it, by label.
    pub reaches: Vec<String>,
}

/// A spline's length as the file wrote it, beside the curve's own.
///
/// The file's number is a cache that falls behind when the curve moves, so
/// nothing in the product reads it; an evaluator that trusts it places
/// whatever cites the name somewhere else.
#[derive(Debug, Clone, PartialEq)]
pub struct LengthNote {
    /// The `Spl_` name formulas cite the length by.
    pub name: String,
    /// The number the file wrote, in centimetres.
    pub written: f64,
    /// The curve's arc length for the imported body, in centimetres.
    pub arc: f64,
    /// The points whose formulas cite the name, by name, in file order.
    pub cited_by: Vec<String>,
}

/// A piece, as the product carries it and as it does not.
#[derive(Debug, Clone, PartialEq)]
pub struct PieceNote {
    /// Its name.
    pub name: String,
    /// The letter on its label.
    pub letter: String,
    /// How many to cut.
    pub quantity: u32,
    /// Whether the label says it is cut on the fold.
    pub on_fold: bool,
    /// The seam allowance in centimetres, if it has one.
    pub seam_allowance: Option<f64>,
    /// The label's lines.
    pub labels: Vec<String>,
    /// Corners of the product's contour.
    pub nodes: usize,
    /// Curved tracts among them, with how many samples each is flattened
    /// at and how far that flattening strays from the curve, in centimetres.
    pub curves: Vec<(String, u16, f64)>,
    /// The direction the contour runs in.
    pub winding: Winding,
    /// The notches, which the product carries by place and count only.
    pub notches: Vec<NotchNote>,
    /// The internal paths, which the product has no place for.
    pub internal: Vec<InternalNote>,
    /// The layout offset the file gives the piece.
    pub placement: Xy,
}

/// A notch of the pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct NotchNote {
    /// The corner it is cut at.
    pub at: String,
    /// Its shape as the file names it.
    pub kind: String,
    /// Its depth into the seam allowance, in centimetres.
    pub length: f64,
}

/// An internal path of the pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct InternalNote {
    /// Its name.
    pub name: String,
    /// Its stroke as the file names it.
    pub line_type: String,
    /// The file's `cut` flag.
    pub cut: bool,
}
