use std::f64::consts::FRAC_PI_2;

use serde::{Deserialize, Serialize};

use crate::PointKey;

/// A pattern piece: its ordered contour and the grain it is cut on.
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
}

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
    pub samples: u16,
}

/// What runs between one node and the next.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Segment {
    /// A straight line.
    Line,
    /// A cubic whose two handles are points of the document like any other.
    Cubic {
        /// The handle leaving this node.
        out: PointKey,
        /// The handle entering the next node.
        into: PointKey,
    },
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
}

impl Segment {
    /// Whether the tract's handles name `point`.
    pub fn cites(self, point: PointKey) -> bool {
        match self {
            Segment::Line => false,
            Segment::Cubic { out, into } => out == point || into == point,
        }
    }

    /// The two handles, when the tract has them.
    pub fn handles(self) -> Option<(PointKey, PointKey)> {
        match self {
            Segment::Line => None,
            Segment::Cubic { out, into } => Some((out, into)),
        }
    }
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
mod tests {
    #![allow(clippy::float_cmp, reason = "a grain stores the angle it was given")]

    use super::*;

    fn keys() -> Vec<PointKey> {
        (0..4).map(|index| PointKey::new(index, 0)).collect()
    }

    #[test]
    fn a_polygon_is_all_straight_tracts() {
        let piece = Piece::polygon("Delantero", keys(), Winding::Cw);
        assert_eq!(piece.contour.len(), 4);
        assert!(piece.contour.iter().all(|node| node.samples == 1));
        assert!(
            piece
                .contour
                .iter()
                .all(|node| node.segment == Segment::Line)
        );
        assert_eq!(piece.grain, Grain::VERTICAL);
    }

    #[test]
    fn a_node_is_found_by_its_point_never_by_an_index() {
        let keys = keys();
        let piece = Piece::polygon("Delantero", keys.clone(), Winding::Cw);
        assert_eq!(piece.node_index(keys[2]), Some(2));
        assert_eq!(piece.node_index(PointKey::new(9, 0)), None);
    }

    #[test]
    fn a_handle_is_cited_by_the_piece_that_holds_its_tract() {
        let keys = keys();
        let mut piece = Piece::polygon("Delantero", keys.clone(), Winding::Cw);
        let handle = PointKey::new(7, 0);
        piece.contour[1].segment = Segment::Cubic {
            out: handle,
            into: keys[3],
        };
        assert!(piece.cites(handle));
        assert!(!piece.cites(PointKey::new(8, 0)));
        assert_eq!(piece.node_index(handle), None);
    }

    #[test]
    fn a_clockwise_contour_on_the_page_has_a_positive_area() {
        assert_eq!(Winding::of_area(5230.39), Winding::Cw);
        assert_eq!(Winding::of_area(-1.0), Winding::Ccw);
    }
}
