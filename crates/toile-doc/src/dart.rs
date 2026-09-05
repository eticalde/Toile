use serde::{Deserialize, Serialize};

use crate::{ContourNode, Identity, PieceKey, Point, PointKey, SeamKey, Segment};

/// A dart: the wedge taken out of a contour, and the seam that closes it.
///
/// The apex and the two legs are ordinary contour nodes, so the wedge leaves a
/// simple polygon the mesher already knows how to fill, and closing the dart
/// is the seam between the two legs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Dart {
    /// The point the wedge closes onto.
    pub apex: PointKey,
    /// The two sides of the wedge, in contour order.
    pub legs: (PointKey, PointKey),
    /// The seam that sews one leg to the other.
    pub seam: SeamKey,
    /// Which way the folded wedge lies once the dart is sewn.
    pub fold: FoldDirection,
}

/// Which way a sewn dart is pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoldDirection {
    /// Toward the start of the contour.
    TowardStart,
    /// Toward its end.
    TowardEnd,
}

/// The cut a dart makes in its piece: three nodes, and where they go.
///
/// The command that adds a dart carries its wedge, so the dart and the notch
/// it cuts are one entry of the history and never half of one.
#[derive(Debug, Clone, PartialEq)]
pub struct DartWedge {
    /// The piece whose contour the wedge cuts.
    pub piece: PieceKey,
    /// The node the wedge follows; `None` opens the contour at its head.
    pub after: Option<PointKey>,
    /// First leg, apex and second leg, in contour order.
    pub nodes: [WedgeNode; 3],
}

/// One node of a wedge: which point it is, and the tract that leaves it.
#[derive(Debug, Clone, PartialEq)]
pub struct WedgeNode {
    /// The key the point takes: a fresh one, or the one undo gives back.
    pub identity: Identity<Point>,
    /// The point itself.
    pub value: Point,
    /// The tract leaving the node.
    pub segment: Segment,
    /// How many samples that tract is flattened into.
    pub samples: u16,
}

impl WedgeNode {
    /// A wedge node whose tract is a straight line.
    pub fn line(identity: Identity<Point>, value: Point) -> WedgeNode {
        WedgeNode {
            identity,
            value,
            segment: Segment::Line,
            samples: 1,
        }
    }

    /// The contour node this becomes once its point has a key.
    pub fn node(&self, point: PointKey) -> ContourNode {
        ContourNode {
            point,
            segment: self.segment,
            samples: self.samples,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wedge_node_becomes_a_contour_node_once_it_has_a_key() {
        let wedge = WedgeNode::line(Identity::New, Point::at(0.0, 0.0));
        let node = wedge.node(PointKey::new(4, 0));
        assert_eq!(node.point, PointKey::new(4, 0));
        assert_eq!(node.segment, Segment::Line);
        assert_eq!(node.samples, 1);
    }
}
