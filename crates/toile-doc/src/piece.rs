use std::f64::consts::FRAC_PI_2;

use serde::{Deserialize, Serialize};

use crate::{DocError, Placement, PointKey, Segment};

/// A pattern piece: its ordered contour, the grain it is cut on, and what its
/// author wrote about cutting it out.
///
/// The closure is implicit: the last node's tract runs back to the first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Piece {
    /// The name the product tree shows; unique in the document.
    pub name: String,
    /// The contour, in order.
    pub contour: Vec<ContourNode>,
    /// The direction the contour runs in, declared rather than deduced.
    pub winding: Winding,
    /// The grain line the piece is cut on.
    #[serde(default)]
    pub grain: Grain,
    /// How far outside the drawn line the cloth is cut, in centimetres.
    ///
    /// `None` is a piece cut on its own line, which a real trouser has two of:
    /// a strip folded in thirds is sewn to nothing along its length, so an
    /// allowance there would be cloth nobody asked for. The paper says this
    /// number; nothing offsets the contour by it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seam_allowance: Option<f64>,
    /// How many of the piece the garment takes.
    #[serde(default = "once", skip_serializing_if = "is_once")]
    pub quantity: u32,
    /// The letter its label shows, so a printed sheet names the piece on it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub letter: Option<String>,
    /// The lines of its label, as its author wrote them.
    ///
    /// Prose, kept whole and never read for a number. A label reading
    /// `cortar 2 + entretela` on a piece whose count says 2 is one author
    /// saying two things, and the document carries both of them rather than
    /// choosing which one it believes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    /// Where the product overview draws the piece, once someone arranged it.
    ///
    /// Absent, the overview lays the piece out itself and nothing is written,
    /// which is what keeps a product nobody arranged in the format version it
    /// had.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<Placement>,
}

/// The narrowest and the widest a bending tract may be flattened to.
///
/// The floor is two, because a curve flattened at one sample is its own
/// chord: a straight tract wearing handles, drawn and meshed and measured as
/// the line it says it is not.
///
/// The ceiling is low on purpose, and it is a refusal rather than a clamp. A
/// whole piece is meshed at a few hundred boundary samples, so one tract past
/// this asks for a resolution no cloth carries. And the count comes off a
/// file: the flattening it sizes is scanned pairwise, twice over, by the
/// contour check every resolve runs, so an unbounded count is a file choosing
/// how long opening it takes. It is the same reasoning `Arena::MAX_ISSUED`
/// is written from.
pub const SAMPLES: (u16, u16) = (2, 96);

/// One node of a contour: a point, and the tract that leaves it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ContourNode {
    /// The point the contour passes through here.
    pub point: PointKey,
    /// The tract from this node to the next.
    pub segment: Segment,
    /// How many samples that tract is flattened into.
    ///
    /// Persisting the count rather than a tolerance is what keeps adjusting a
    /// curve a change of shape: the number of points cannot move under it.
    /// What it may hold is `SAMPLES`, and `takes_samples` is where that is
    /// asked.
    pub samples: u16,
}

/// The direction a contour runs in, as it is drawn on the table.
///
/// The document's y grows downward, so a contour drawn clockwise on the page
/// has a positive signed area in document coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Winding {
    /// Counterclockwise on the page.
    Ccw,
    /// Clockwise on the page.
    Cw,
}

/// The grain line of a piece: the direction of the warp on the cut cloth.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "radians", rename_all = "lowercase")]
pub enum Grain {
    /// Radians from the x axis toward the y axis, which is down the page.
    Angle(f64),
}

impl Piece {
    /// How many of a piece are cut when nobody said otherwise.
    ///
    /// One, so that a file written before the count — and every piece whose
    /// author never wrote one — is a piece cut once rather than cut none.
    pub const CUT_ONCE: u32 = 1;

    /// A piece whose contour is the straight polygon through `points`.
    pub fn polygon(
        name: &str,
        points: impl IntoIterator<Item = PointKey>,
        winding: Winding,
    ) -> Piece {
        Piece {
            name: name.to_owned(),
            contour: points.into_iter().map(ContourNode::line).collect(),
            winding,
            grain: Grain::default(),
            seam_allowance: None,
            quantity: Piece::CUT_ONCE,
            letter: None,
            labels: Vec::new(),
            placement: None,
        }
    }

    /// Whether the piece says anything at all about how it is cut out.
    pub fn says_how_it_is_cut(&self) -> bool {
        self.seam_allowance.is_some()
            || self.quantity != Piece::CUT_ONCE
            || self.letter.is_some()
            || !self.labels.is_empty()
    }

    /// The oldest format version whose reader keeps everything the piece
    /// carries: anything about cutting it needs 10, a placement 4, the rest 1.
    pub(crate) fn format_version(&self) -> u32 {
        if self.says_how_it_is_cut() {
            crate::json::VERSION_CUT
        } else if self.placement.is_some() {
            crate::json::VERSION_PLACED
        } else {
            crate::json::VERSION
        }
    }

    /// Where `point` sits in the contour, if the contour runs through it.
    pub fn node_index(&self, point: PointKey) -> Option<usize> {
        self.contour.iter().position(|node| node.point == point)
    }

    /// Whether the contour names `point`, as a node or as a handle.
    pub fn cites(&self, point: PointKey) -> bool {
        self.contour
            .iter()
            .any(|node| node.point == point || node.segment.cites(point))
    }

    /// The points the contour passes through, in contour order.
    pub fn anchors(&self) -> impl Iterator<Item = PointKey> {
        self.contour.iter().map(|node| node.point)
    }
}

impl ContourNode {
    /// A node whose tract is the straight line to the next node.
    pub fn line(point: PointKey) -> ContourNode {
        ContourNode {
            point,
            segment: Segment::Line,
            samples: 1,
        }
    }

    /// Whether the tract leaving this node may be flattened at `count`.
    ///
    /// A straight tract gives its own node and stops whatever its count says,
    /// so one describes it as well as any other number under the ceiling. A
    /// bending one is flattened at exactly the count it names, so it answers
    /// to the whole of `SAMPLES`.
    pub fn takes_samples(&self, count: u16) -> bool {
        samples_fit(self.segment.bends(), count)
    }
}

/// Whether a tract that bends, or does not, may be flattened at `count`.
///
/// The rule lives here rather than on `ContourNode` because the edit that
/// inserts a node has to ask it about a tract the contour does not carry yet.
pub(crate) fn samples_fit(bends: bool, count: u16) -> bool {
    let floor = if bends { SAMPLES.0 } else { 1 };
    (floor..=SAMPLES.1).contains(&count)
}

/// The count a piece carries when the file writes none.
const fn once() -> u32 {
    Piece::CUT_ONCE
}

/// Whether a count is the one the file leaves out.
#[allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde hands a reference to the field it is deciding about"
)]
fn is_once(quantity: &u32) -> bool {
    *quantity == Piece::CUT_ONCE
}

/// Refuses a seam allowance no piece is cut with.
///
/// A width of cloth outside the line, so a negative one takes cloth away from
/// a piece instead of adding it. The numbers JSON cannot spell go out the same
/// door, but not for the reason a placement's do: a placement's `x` is a bare
/// `f64`, so the `null` the writer puts there refuses the file on the next
/// open. This field is an `Option`, and `null` is how `None` is spelled — so
/// the piece comes back *net*, the stamp falls from the cut's version to the
/// first, and nothing says a word. A loud refusal is a worse bug than a quiet
/// one only until the quiet one is the one that costs cloth.
pub(crate) fn check_allowance(seam_allowance: Option<f64>) -> Result<(), DocError> {
    match seam_allowance {
        Some(width) if !width.is_finite() || width < 0.0 => Err(DocError::SeamAllowance),
        _ => Ok(()),
    }
}

/// Refuses a count no garment cuts.
///
/// Zero is the one whole number that is not a count: a piece the garment cuts
/// none of is a piece that does not belong on the table, and a printed sheet
/// that asks for none of it is a sheet nobody can act on.
pub(crate) fn check_quantity(quantity: u32) -> Result<(), DocError> {
    if quantity == 0 {
        return Err(DocError::CutQuantity);
    }
    Ok(())
}

impl Winding {
    /// The direction a closed contour of this signed area runs in.
    ///
    /// The area is the one the shoelace formula gives in document
    /// coordinates, where y grows downward.
    pub fn of_area(area: f64) -> Winding {
        if area > 0.0 {
            Winding::Cw
        } else {
            Winding::Ccw
        }
    }
}

impl Grain {
    /// Straight down the page, which is how a piece is cut unless it is not.
    pub const VERTICAL: Grain = Grain::Angle(FRAC_PI_2);

    /// The angle in radians, from the x axis toward the y axis.
    pub fn radians(self) -> f64 {
        match self {
            Grain::Angle(radians) => radians,
        }
    }
}

impl Default for Grain {
    fn default() -> Grain {
        Grain::VERTICAL
    }
}

#[cfg(test)]
mod tests;
