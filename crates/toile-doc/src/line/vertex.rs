use serde::{Deserialize, Serialize};

use crate::{EdgeAnchor, PointKey};

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
