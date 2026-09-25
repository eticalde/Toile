use serde::{Deserialize, Serialize};

use crate::{EdgeAnchor, Identity, Point, PointKey};

/// One place an internal line runs through.
///
/// The two are not interchangeable, and a pattern needs both. A vertex on the
/// contour follows the cloth: re-draft the piece for another body and the
/// pocket mouth stays the same distance along the same tract, because an
/// anchor is a node key and a fraction local to it. A free vertex is a point
/// of the document, two coordinates bound to formulas, which is how a curve
/// handle already lives — and it is what a buttonhole or a belt-loop mark has
/// to be, since nothing on the contour says where those go.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum LineVertex {
    /// A place on the contour of the piece the line is drawn on.
    Contour(EdgeAnchor),
    /// A point of the document, placed by its own two bindings.
    Free {
        /// The point.
        point: PointKey,
    },
}

impl LineVertex {
    /// The place on a contour, when the vertex sits on one.
    pub fn anchor(self) -> Option<EdgeAnchor> {
        match self {
            LineVertex::Contour(anchor) => Some(anchor),
            LineVertex::Free { .. } => None,
        }
    }

    /// The point of the document, when the vertex is a free one.
    pub fn point(self) -> Option<PointKey> {
        match self {
            LineVertex::Contour(_) => None,
            LineVertex::Free { point } => Some(point),
        }
    }

    /// The vertex sitting on a free point of the document.
    pub fn free(point: PointKey) -> LineVertex {
        LineVertex::Free { point }
    }
}

/// One place of an internal line on its way into the document.
///
/// It stands to [`LineVertex`] as `SegmentEdit` stands to `Segment`, and for
/// the same reason: a free place is a point of the document, so the edit that
/// draws the line is the edit that creates it and the edit that rubs the line
/// out takes it away. The point travels in the command rather than its key
/// alone, which is what lets undo give the very same key back with whatever it
/// had grown into.
///
/// `Cited` is the case a curve has no use for and a line does: a drafting
/// skeleton names places that outlive any one line — a corner two lines meet
/// at, a construction point an import already wrote — and a point somebody else
/// still needs is not the line's to create or to take away.
#[derive(Debug, Clone, PartialEq)]
pub enum VertexEdit {
    /// A place on the contour of the piece the line is drawn on.
    Contour(EdgeAnchor),
    /// A place of its own, as the point this drawing puts into the document.
    Free {
        /// A key the arena has not issued yet, or the one undo gives back.
        identity: Identity<Point>,
        /// The point itself, bindings and name included.
        value: Point,
    },
    /// A place of its own, on a point the document already carries.
    Cited(PointKey),
}

impl VertexEdit {
    /// The place on a contour, when the place sits on one.
    pub fn anchor(&self) -> Option<EdgeAnchor> {
        match self {
            VertexEdit::Contour(anchor) => Some(*anchor),
            VertexEdit::Free { .. } | VertexEdit::Cited(_) => None,
        }
    }

    /// A place of its own, on a point the drawing creates.
    pub fn free(value: Point) -> VertexEdit {
        VertexEdit::Free {
            identity: Identity::New,
            value,
        }
    }

    /// The same, taking back the key the point carried before it was removed.
    pub fn restored(key: PointKey, value: Point) -> VertexEdit {
        VertexEdit::Free {
            identity: Identity::Restored(key),
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "a vertex stores the fraction it was given"
    )]

    use super::*;
    use crate::PieceKey;

    fn anchored() -> LineVertex {
        LineVertex::Contour(EdgeAnchor {
            piece: PieceKey::new(0, 0),
            from: PointKey::new(3, 0),
            t: 0.25,
        })
    }

    #[test]
    fn a_vertex_answers_for_exactly_one_of_the_two_places_it_can_be() {
        assert_eq!(anchored().point(), None);
        assert_eq!(anchored().anchor().map(|at| at.t), Some(0.25));
        let free = LineVertex::free(PointKey::new(7, 0));
        assert_eq!(free.point(), Some(PointKey::new(7, 0)));
        assert_eq!(free.anchor(), None);
    }

    /// A place on its way in answers for the same two places, and says besides
    /// whether the drawing is the one that creates its point.
    #[test]
    fn a_place_on_its_way_in_says_whether_it_brings_its_own_point() {
        let key = PointKey::new(7, 0);
        let drawn = VertexEdit::free(Point::at(2.0, 3.0));
        assert_eq!(drawn.anchor(), None);
        assert_eq!(
            drawn,
            VertexEdit::Free {
                identity: Identity::New,
                value: Point::at(2.0, 3.0)
            }
        );
        let back = VertexEdit::restored(key, Point::at(2.0, 3.0));
        assert_eq!(
            back,
            VertexEdit::Free {
                identity: Identity::Restored(key),
                value: Point::at(2.0, 3.0)
            }
        );
        assert_eq!(VertexEdit::Cited(key).anchor(), None);
        let anchor = anchored().anchor().expect("it sits on a contour");
        assert_eq!(VertexEdit::Contour(anchor).anchor(), Some(anchor));
    }

    /// The anchor's own fields sit beside the tag rather than under a second
    /// object, which is how a seam's ends and a notch already read in a file.
    #[test]
    fn a_vertex_writes_its_place_beside_the_word_for_which_place_it_is() {
        let written = serde_json::to_string(&anchored()).expect("a vertex writes");
        assert_eq!(
            written,
            "{\"kind\":\"contour\",\"piece\":\"0.0\",\"from\":\"3.0\",\"t\":0.25}"
        );
        let free =
            serde_json::to_string(&LineVertex::free(PointKey::new(7, 0))).expect("a vertex writes");
        assert_eq!(free, "{\"kind\":\"free\",\"point\":\"7.0\"}");
        for text in [written.as_str(), free.as_str()] {
            let read: LineVertex = serde_json::from_str(text).expect("what was written reads");
            let again = serde_json::to_string(&read).expect("a vertex writes");
            assert_eq!(again, text);
        }
    }
}
