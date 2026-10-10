use crate::{Id, Xy};

/// A line drawn inside a piece: a slit, a stitching line, a fold, a guide.
#[derive(Debug, Clone, PartialEq)]
pub struct InternalPath {
    /// Its id, which pieces cite.
    pub id: Id,
    /// The author's name for it.
    pub name: String,
    /// The stroke, e.g. `solidLine` or `dashLine`.
    pub line_type: String,
    /// The file's `cut` flag for the path, as written.
    pub cut: bool,
    /// What it walks, in order.
    pub nodes: Vec<PathNode>,
}

/// What a path node walks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    /// A point.
    Point,
    /// A spline.
    Spline,
    /// A spline path.
    SplinePath,
    /// An arc.
    Arc,
}

/// One step of a path: a construction object, walked forward or reversed.
#[derive(Debug, Clone, PartialEq)]
pub struct PathNode {
    /// What the step walks.
    pub kind: NodeKind,
    /// The construction object, reached through the modeling copy the file
    /// cites.
    pub object: Id,
    /// Whether a curve is walked from its end back to its start.
    pub reverse: bool,
    /// The notch cut at this node of a piece outline.
    pub notch: Option<Notch>,
}

/// A notch at a node of a piece outline.
#[derive(Debug, Clone, PartialEq)]
pub struct Notch {
    /// Its shape as the file names it, e.g. `slit`.
    pub kind: String,
    /// Its depth into the seam allowance, in centimetres.
    pub length: f64,
}

/// A cut piece.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    /// Its id.
    pub id: Id,
    /// Its name.
    pub name: String,
    /// The seam allowance in centimetres, `None` when the piece is cut on
    /// its line.
    pub seam_allowance: Option<f64>,
    /// The letter on its label.
    pub letter: String,
    /// How many to cut.
    pub quantity: u32,
    /// Whether it is cut on the fold.
    pub on_fold: bool,
    /// The label's text lines.
    pub labels: Vec<String>,
    /// The angle of its grain line in degrees, as the file writes an angle:
    /// zero east, counter-clockwise on a page whose y grows downward.
    ///
    /// `None` when the file writes none, which is a piece whose author never
    /// placed a grain line on it rather than one cut along the x axis.
    pub grain: Option<f64>,
    /// The outline, in walking order.
    pub outline: Vec<PathNode>,
    /// The internal paths drawn on it.
    pub internal_paths: Vec<Id>,
    /// How far the piece is moved from where its construction draws it when
    /// the pieces are laid out, in centimetres: the file's `mx` and `my`.
    pub placement: Xy,
}
