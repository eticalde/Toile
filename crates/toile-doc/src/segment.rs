use serde::{Deserialize, Serialize};

use crate::{Identity, Point, PointKey};

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

/// Which end of a tract a handle belongs to.
///
/// It sits beside [`Segment`] rather than in the interface because it is
/// [`Segment::handle`] that has to be told which one is wanted. A cubic's two
/// handles are the same type and lie next to each other, so asking for one by
/// tuple position is a press taking hold of the tangent of a different node
/// and dragging it, one binding written per frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// It leaves the node the tract starts at.
    Out,
    /// It enters the node the tract ends at.
    Into,
}

/// What a tract is to become, with the handle points a curve brings with it.
///
/// A handle is a point of the document like any other, so the edit that draws
/// a curve is the edit that creates its two handles and the edit that
/// straightens the tract is the one that takes them away. The points travel in
/// the command rather than their keys alone, which is what lets undo give the
/// very same keys back with whatever the handles had grown into: a position, a
/// formula, a name.
#[derive(Debug, Clone, PartialEq)]
pub enum SegmentEdit {
    /// A straight line: the handles the tract had leave the document.
    Line,
    /// A cubic, with the two handles it puts into the document.
    ///
    /// They are boxed because a handle carries a whole point, formulas and
    /// all, and every command of the undo stack would otherwise be as wide as
    /// the widest one.
    Cubic(Box<Handles>),
}

/// The two handles a cubic hangs on, on their way into the document.
#[derive(Debug, Clone, PartialEq)]
pub struct Handles {
    /// The handle leaving the node.
    pub out: Handle,
    /// The handle entering the next node.
    pub into: Handle,
}

/// A handle on its way into the document: the key it takes, and the point.
#[derive(Debug, Clone, PartialEq)]
pub struct Handle {
    /// A key the arena has not issued yet, or the one undo gives back.
    pub identity: Identity<Point>,
    /// The point itself, bindings and name included.
    pub value: Point,
}

impl Segment {
    /// Whether the tract bends, which is what decides the fewest samples it
    /// can be flattened at.
    pub fn bends(self) -> bool {
        matches!(self, Segment::Cubic { .. })
    }

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

    /// The handle on one side of the tract, when the tract has handles.
    ///
    /// For a caller that wants one of the two rather than both: it says which
    /// in the word the variant already uses, instead of counting.
    pub fn handle(self, side: Side) -> Option<PointKey> {
        match (self, side) {
            (Segment::Line, _) => None,
            (Segment::Cubic { out, .. }, Side::Out) => Some(out),
            (Segment::Cubic { into, .. }, Side::Into) => Some(into),
        }
    }
}

impl SegmentEdit {
    /// Whether the tract this edit makes bends.
    pub fn bends(&self) -> bool {
        matches!(self, SegmentEdit::Cubic(_))
    }

    /// A curve on two handles, whatever keys they are taking.
    pub fn curve(out: Handle, into: Handle) -> SegmentEdit {
        SegmentEdit::Cubic(Box::new(Handles { out, into }))
    }

    /// A curve drawn on two handles the document does not carry yet.
    pub fn cubic(out: Point, into: Point) -> SegmentEdit {
        SegmentEdit::curve(Handle::new(out), Handle::new(into))
    }
}

impl Handle {
    /// A handle the arena has yet to give a key to.
    pub fn new(value: Point) -> Handle {
        Handle {
            identity: Identity::New,
            value,
        }
    }

    /// A handle taking back the key it carried before it was removed.
    pub fn restored(key: PointKey, value: Point) -> Handle {
        Handle {
            identity: Identity::Restored(key),
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_straight_tract_has_no_handles() {
        assert_eq!(Segment::Line.handles(), None);
        assert!(!Segment::Line.cites(PointKey::new(0, 0)));
    }

    #[test]
    fn a_curve_names_both_of_its_handles() {
        let curve = Segment::Cubic {
            out: PointKey::new(4, 0),
            into: PointKey::new(5, 0),
        };
        assert_eq!(
            curve.handles(),
            Some((PointKey::new(4, 0), PointKey::new(5, 0)))
        );
        assert!(curve.cites(PointKey::new(5, 0)));
        assert!(!curve.cites(PointKey::new(6, 0)));
    }

    #[test]
    fn a_handle_is_asked_for_by_the_side_it_hangs_on() {
        let curve = Segment::Cubic {
            out: PointKey::new(4, 0),
            into: PointKey::new(5, 0),
        };
        assert_eq!(curve.handle(Side::Out), Some(PointKey::new(4, 0)));
        assert_eq!(curve.handle(Side::Into), Some(PointKey::new(5, 0)));
        assert_eq!(Segment::Line.handle(Side::Out), None);
    }

    #[test]
    fn a_curve_is_drawn_on_points_the_document_has_no_key_for_yet() {
        let edit = SegmentEdit::cubic(Point::at(1.0, 2.0), Point::at(3.0, 4.0));
        let SegmentEdit::Cubic(handles) = edit else {
            panic!("the edit draws a curve")
        };
        assert_eq!(handles.out.identity, Identity::New);
        assert_eq!(handles.into.value, Point::at(3.0, 4.0));
    }
}
