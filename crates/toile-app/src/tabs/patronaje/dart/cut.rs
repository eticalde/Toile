use toile_engine::draft::{Dart, DartKey, Doc, Draft, FoldDirection, PieceKey};

/// One dart of a piece, as the panel and the mat read it.
#[derive(Debug, Clone, PartialEq)]
pub struct Cut {
    /// The dart.
    pub dart: DartKey,
    /// The number the panel lists it under, among the darts of its piece.
    pub ordinal: usize,
    /// What the drawing calls its first leg, its apex and its second leg.
    pub names: [String; 3],
    /// Which way the sewn wedge is pressed.
    pub fold: FoldDirection,
    /// Its first leg, its apex and its second leg, in centimetres.
    ///
    /// Nothing at all when one of the three resolves nowhere, which is what
    /// keeps the panel from measuring a wedge the mat is not drawing.
    pub at: Option<[[f64; 2]; 3]>,
}

impl Cut {
    /// The name of the leg the folded wedge lies against.
    pub fn toward(&self) -> &str {
        match self.fold {
            FoldDirection::TowardStart => &self.names[0],
            FoldDirection::TowardEnd => &self.names[2],
        }
    }

    /// How wide the mouth is: leg to leg in a straight line, in centimetres.
    ///
    /// The straight line and not the contour between them, because there is no
    /// contour between them any more — the wedge is what the cut took out, and
    /// this is the length the waist run loses when the dart closes.
    pub fn mouth_cm(&self) -> Option<f64> {
        let at = self.at?;
        Some(span(at[0], at[2]))
    }

    /// How long each leg is, in centimetres: what the run gains in their place.
    pub fn legs_cm(&self) -> Option<[f64; 2]> {
        let at = self.at?;
        Some([span(at[0], at[1]), span(at[1], at[2])])
    }
}

/// Every dart cut into a piece, in key order.
pub fn on(draft: &Draft, piece: PieceKey) -> Vec<Cut> {
    let doc = draft.doc();
    let mine: Vec<(DartKey, Dart)> = doc
        .darts
        .iter()
        .filter(|&(_, held)| cut_into(doc, held) == Some(piece))
        .map(|(key, held)| (key, *held))
        .collect();
    mine.into_iter()
        .enumerate()
        .map(|(rank, (key, held))| {
            let points = [held.legs.0, held.apex, held.legs.1];
            let named = |at: usize| doc.label_of(piece, points[at]).unwrap_or_default();
            Cut {
                dart: key,
                ordinal: rank + 1,
                names: [named(0), named(1), named(2)],
                fold: held.fold,
                at: resolved(draft, points),
            }
        })
        .collect()
}

/// The piece a dart is cut into, which is what its seam names.
///
/// The record names three points, and a point on its own does not name the
/// contour it is on. It is the same question the document asks when a dart is
/// taken off.
pub fn cut_into(doc: &Doc, dart: &Dart) -> Option<PieceKey> {
    doc.seams.get(dart.seam)?.a.piece()
}

/// Where the three points of a wedge lie, when all three of them lie anywhere.
fn resolved(draft: &Draft, points: [toile_engine::draft::PointKey; 3]) -> Option<[[f64; 2]; 3]> {
    let at = points.map(|key| draft.resolved(key));
    Some([at[0]?, at[1]?, at[2]?])
}

/// The straight distance between two places, in centimetres.
fn span(from: [f64; 2], to: [f64; 2]) -> f64 {
    (to[0] - from[0]).hypot(to[1] - from[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A wedge on a 5-12-13 triangle: a mouth ten wide and two legs of
    /// thirteen.
    fn wedge() -> Cut {
        Cut {
            dart: DartKey::new(0, 0),
            ordinal: 1,
            names: ["a".to_owned(), "pico".to_owned(), "b".to_owned()],
            fold: FoldDirection::TowardStart,
            at: Some([[15.0, 0.0], [20.0, 12.0], [25.0, 0.0]]),
        }
    }

    /// The mouth and the legs are the two numbers a waist run is read with: the
    /// run loses the mouth and gains both legs when the dart closes.
    #[test]
    fn a_wedge_measures_its_mouth_and_each_of_its_legs() {
        let cut = wedge();
        assert!((cut.mouth_cm().expect("it resolves") - 10.0).abs() < 1.0e-12);
        let legs = cut.legs_cm().expect("it resolves");
        assert!((legs[0] - 13.0).abs() < 1.0e-12, "{legs:?}");
        assert!((legs[1] - 13.0).abs() < 1.0e-12, "{legs:?}");
    }

    /// Which leg the dart is pressed against is named, and it is the other one
    /// when it is pressed the other way.
    #[test]
    fn the_panel_names_the_leg_the_wedge_is_pressed_against() {
        assert_eq!(wedge().toward(), "a");
        let other = Cut {
            fold: FoldDirection::TowardEnd,
            ..wedge()
        };
        assert_eq!(other.toward(), "b");
    }

    /// A wedge one of whose points resolves nowhere measures nothing rather
    /// than measuring against a place nobody drew.
    #[test]
    fn a_wedge_that_does_not_resolve_measures_nothing() {
        let adrift = Cut {
            at: None,
            ..wedge()
        };
        assert!(adrift.mouth_cm().is_none());
        assert!(adrift.legs_cm().is_none());
    }
}
