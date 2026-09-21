use std::collections::BTreeMap;

use crate::{Formula, InternalPath, Piece, Place};

/// Construction objects, one element at a time.
mod calc;
/// Every id read so far, and what it names.
mod index;
/// Modeling copies and the internal paths built from them.
mod modeling;
/// Cut pieces and their outlines.
mod pieces;
/// The pattern file, its header and its blocks.
mod read;

/// A Seamly object id. One id space covers the whole file.
pub type Id = u32;

/// A Seamly pattern, read into its construction graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    /// The file format version, e.g. `0.7.4`.
    pub version: String,
    /// The measurement file the pattern names, as written: usually relative
    /// to the pattern's own folder.
    pub measurements: Option<String>,
    /// The pattern's variables, in file order.
    pub variables: Vec<Variable>,
    /// The draft blocks, in file order.
    pub blocks: Vec<Block>,
    /// Where the file carries a comment, in file order.
    pub comments: Vec<Comment>,
}

/// Where a comment sits: free text for people, which no construction reads.
///
/// Its text is not kept, so nothing downstream can copy the author's notes
/// anywhere by accident.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// The 1-based line it starts on.
    pub line: u32,
    /// The draft block it sits in; `None` outside every block.
    pub block: Option<String>,
    /// Whether it is the note Seamly stamps on every file it saves, rather
    /// than one its author wrote.
    pub signature: bool,
}

/// A quantity the pattern names once, written `#name`. Its formula may cite
/// measurements and the variables above it.
#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    /// The name, `#` included.
    pub name: String,
    /// The value as written.
    pub formula: Formula,
    /// The author's note on it.
    pub description: String,
    /// Where the file defines it.
    pub at: Place,
}

/// One draft block: its construction, the copies its paths are built from,
/// and its pieces.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The block's name.
    pub name: String,
    /// The construction, in file order. Each object cites only objects above
    /// it, in this block or an earlier one.
    pub objects: Vec<Object>,
    /// The modeling copies: the id a path cites, mapped to the construction
    /// object it copies.
    pub copies: BTreeMap<Id, Id>,
    /// The lines drawn inside pieces: slits, stitching, fold and guide lines.
    pub internal_paths: Vec<InternalPath>,
    /// The cut pieces.
    pub pieces: Vec<Piece>,
}

/// One construction object.
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    /// Its id.
    pub id: Id,
    /// Where the file defines it.
    pub at: Place,
    /// What it is and how it is built.
    pub kind: ObjectKind,
}

/// The construction objects this crate models.
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectKind {
    /// A named point.
    Point {
        /// The name formulas and drawings know it by.
        name: String,
        /// How it is placed.
        construction: Construction,
        /// The stroke drawn from the point it is built on, if the
        /// construction draws one; `none` hides it.
        line_type: Option<String>,
    },
    /// A segment between two points. It defines `Line_a_b` and
    /// `AngleLine_a_b` for them, whatever its stroke.
    Line {
        /// One end.
        first: Id,
        /// The other end.
        second: Id,
        /// The stroke; `none` hides it.
        line_type: String,
    },
    /// A cubic curve between two points.
    Spline(Spline),
    /// A chain of cubic curves through points.
    SplinePath(Vec<PathPoint>),
    /// An arc of a circle.
    Arc(Arc),
}

/// How a point is placed. Angles are in degrees, 0 east and counter-clockwise
/// as seen on the page, whose y axis grows downward.
#[derive(Debug, Clone, PartialEq)]
pub enum Construction {
    /// At coordinates of its own.
    Single {
        /// Abscissa.
        x: Formula,
        /// Ordinate, growing downward.
        y: Formula,
    },
    /// At `length` from `base`, heading `angle`.
    EndLine {
        /// The point it is built on.
        base: Id,
        /// Distance from `base`.
        length: Formula,
        /// Heading from `base`.
        angle: Formula,
    },
    /// At `length` from `first` toward `second`. The formula may cite
    /// `CurrentLength`, the distance between the two.
    AlongLine {
        /// Where the measure starts.
        first: Id,
        /// The point that gives the direction.
        second: Id,
        /// Distance from `first`.
        length: Formula,
    },
    /// Where the line through `line1` crosses the line through `line2`.
    LineIntersect {
        /// Two points of the first line.
        line1: [Id; 2],
        /// Two points of the second line.
        line2: [Id; 2],
    },
    /// On `spline`, at `length` of arc from its start.
    CutSpline {
        /// The curve it cuts.
        spline: Id,
        /// Arc length from the curve's start.
        length: Formula,
    },
}

/// A cubic Bezier between two points, each handle given as a heading and a
/// length from its own end point.
#[derive(Debug, Clone, PartialEq)]
pub struct Spline {
    /// The first point.
    pub start: Id,
    /// The last point.
    pub end: Id,
    /// Heading of the handle leaving `start`.
    pub angle1: Formula,
    /// Length of the handle leaving `start`.
    pub length1: Formula,
    /// Heading of the handle arriving at `end`, from `end`.
    pub angle2: Formula,
    /// Length of the handle arriving at `end`.
    pub length2: Formula,
    /// The curve's length as the file last wrote it: a cache that goes stale
    /// when the curve moves, never a definition.
    pub written_length: Option<f64>,
}

/// A point of a spline path and its two handles.
#[derive(Debug, Clone, PartialEq)]
pub struct PathPoint {
    /// The point the path passes through.
    pub point: Id,
    /// Heading of the handle arriving at the point.
    pub angle1: Formula,
    /// Length of the handle arriving at the point.
    pub length1: Formula,
    /// Heading of the handle leaving the point.
    pub angle2: Formula,
    /// Length of the handle leaving the point.
    pub length2: Formula,
}

/// An arc of a circle, from `angle1` counter-clockwise to `angle2`.
#[derive(Debug, Clone, PartialEq)]
pub struct Arc {
    /// The centre.
    pub center: Id,
    /// The radius.
    pub radius: Formula,
    /// Where the arc starts.
    pub angle1: Formula,
    /// Where the arc ends.
    pub angle2: Formula,
}

impl Pattern {
    /// Every construction object of every block, in file order.
    pub fn objects(&self) -> impl Iterator<Item = &Object> {
        self.blocks.iter().flat_map(|block| &block.objects)
    }

    /// The construction object with `id`.
    pub fn object(&self, id: Id) -> Option<&Object> {
        self.objects().find(|object| object.id == id)
    }
}

impl Object {
    /// The point's name, if the object is a point.
    pub fn name(&self) -> Option<&str> {
        match &self.kind {
            ObjectKind::Point { name, .. } => Some(name),
            _ => None,
        }
    }

    /// Every formula the object is built from.
    pub fn formulas(&self) -> Vec<&Formula> {
        match &self.kind {
            ObjectKind::Point { construction, .. } => match construction {
                Construction::Single { x, y } => vec![x, y],
                Construction::EndLine { length, angle, .. } => vec![length, angle],
                Construction::AlongLine { length, .. } | Construction::CutSpline { length, .. } => {
                    vec![length]
                }
                Construction::LineIntersect { .. } => Vec::new(),
            },
            ObjectKind::Line { .. } => Vec::new(),
            ObjectKind::Spline(spline) => vec![
                &spline.angle1,
                &spline.length1,
                &spline.angle2,
                &spline.length2,
            ],
            ObjectKind::SplinePath(points) => points
                .iter()
                .flat_map(|p| [&p.angle1, &p.length1, &p.angle2, &p.length2])
                .collect(),
            ObjectKind::Arc(arc) => vec![&arc.radius, &arc.angle1, &arc.angle2],
        }
    }
}
