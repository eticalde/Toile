use crate::{PieceKey, PointKey};

/// A place on a contour, said as a node plus a fraction of the tract that
/// leaves it.
///
/// The fraction is local on purpose. A fraction of the whole perimeter drifts
/// the moment any other tract changes length; a node is a key, and a key does
/// not move.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeAnchor {
    /// The piece the contour belongs to.
    pub piece: PieceKey,
    /// The node the tract leaves from.
    pub from: PointKey,
    /// How far along that tract, from 0 at `from` to 1 at the next node.
    pub t: f64,
}

/// A stretch of contour, from one anchor to another along the contour order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeRange {
    /// Where the stretch starts.
    pub head: EdgeAnchor,
    /// Where the stretch ends.
    pub tail: EdgeAnchor,
}

impl EdgeAnchor {
    /// The anchor sitting on a node itself.
    pub fn at_node(piece: PieceKey, from: PointKey) -> EdgeAnchor {
        EdgeAnchor {
            piece,
            from,
            t: 0.0,
        }
    }

    /// Whether the fraction is one the contour can answer for.
    pub fn is_valid(self) -> bool {
        (0.0..=1.0).contains(&self.t)
    }
}

impl EdgeRange {
    /// A stretch from one node to another, node to node.
    pub fn between(piece: PieceKey, head: PointKey, tail: PointKey) -> EdgeRange {
        EdgeRange {
            head: EdgeAnchor::at_node(piece, head),
            tail: EdgeAnchor::at_node(piece, tail),
        }
    }

    /// The piece the stretch runs on, when both ends agree on one.
    pub fn piece(self) -> Option<PieceKey> {
        (self.head.piece == self.tail.piece).then_some(self.head.piece)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "an anchor stores the fraction it was given"
    )]

    use super::*;

    fn piece() -> PieceKey {
        PieceKey::new(0, 0)
    }

    #[test]
    fn an_anchor_on_a_node_sits_at_the_head_of_its_tract() {
        let anchor = EdgeAnchor::at_node(piece(), PointKey::new(1, 0));
        assert!(anchor.is_valid());
        assert_eq!(anchor.t, 0.0);
    }

    #[test]
    fn a_fraction_outside_the_tract_is_not_valid() {
        let mut anchor = EdgeAnchor::at_node(piece(), PointKey::new(1, 0));
        anchor.t = 1.5;
        assert!(!anchor.is_valid());
        anchor.t = f64::NAN;
        assert!(!anchor.is_valid());
    }

    #[test]
    fn a_range_across_two_pieces_names_neither() {
        let range = EdgeRange::between(piece(), PointKey::new(1, 0), PointKey::new(2, 0));
        assert_eq!(range.piece(), Some(piece()));
        let split = EdgeRange {
            tail: EdgeAnchor::at_node(PieceKey::new(1, 0), PointKey::new(2, 0)),
            ..range
        };
        assert_eq!(split.piece(), None);
    }
}
