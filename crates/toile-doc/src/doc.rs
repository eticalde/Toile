use serde::{Deserialize, Serialize};

use crate::{
    Arena, Dart, MannequinKey, MeasureSet, Notch, Piece, PieceKey, Pin, Point, PointKey, Seam,
    Symmetry, Variable, VariableKey,
};

/// The document: everything a pattern file holds.
///
/// The document and its formulas are in centimetres, always. The ruler's unit
/// is a matter of view, and the metres the solver wants are made once, on the
/// way out.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Doc {
    /// The pieces on the table.
    pub pieces: Arena<Piece>,
    /// Every control point, whichever piece cites it.
    pub points: Arena<Point>,
    /// The seams between stretches of contour.
    pub seams: Arena<Seam>,
    /// The notches on the contours.
    pub notches: Arena<Notch>,
    /// The darts and their wedges.
    pub darts: Arena<Dart>,
    /// The axes pieces are folded or mirrored on.
    pub symmetries: Arena<Symmetry>,
    /// The pins that hold cloth in the viewer.
    pub pins: Arena<Pin>,
    /// The pattern's own quantities.
    pub variables: Arena<Variable>,
    /// The bodies the pattern can be resolved against.
    pub mannequins: Arena<MeasureSet>,
    /// The body it is resolved against right now.
    pub resolve_with: MannequinKey,
}

impl Doc {
    /// An empty document that resolves against `mannequin`.
    ///
    /// It already carries the two tolerances a seam falls back to, so that a
    /// seam created later has something to be judged against.
    pub fn new(mannequin: MeasureSet) -> Doc {
        let mut mannequins = Arena::new();
        let resolve_with = mannequins.insert(mannequin);
        let mut variables = Arena::new();
        variables.insert(Variable::new(
            Seam::TOLERANCE_VARIABLE,
            Seam::DEFAULT_TOLERANCE_CM,
        ));
        variables.insert(Variable::new(
            Seam::RATIO_TOLERANCE_VARIABLE,
            Seam::DEFAULT_RATIO_TOLERANCE,
        ));
        Doc {
            pieces: Arena::new(),
            points: Arena::new(),
            seams: Arena::new(),
            notches: Arena::new(),
            darts: Arena::new(),
            symmetries: Arena::new(),
            pins: Arena::new(),
            variables,
            mannequins,
            resolve_with,
        }
    }

    /// The measurements the pattern resolves against.
    pub fn measures(&self) -> Option<&MeasureSet> {
        self.mannequins.get(self.resolve_with)
    }

    /// Every piece whose contour names `point`, in key order.
    pub fn pieces_citing(&self, point: PointKey) -> Vec<PieceKey> {
        self.pieces
            .iter()
            .filter(|(_, piece)| piece.cites(point))
            .map(|(key, _)| key)
            .collect()
    }

    /// Every piece, in key order.
    pub fn piece_keys(&self) -> Vec<PieceKey> {
        self.pieces.keys().collect()
    }

    /// The name a piece shows for one of its points.
    ///
    /// The label its author wrote, or `P` and its rank in the order the piece
    /// gained its points. Indices are never recycled, so that rank holds still
    /// even after a point is deleted.
    pub fn label_of(&self, piece: PieceKey, point: PointKey) -> Option<String> {
        let held = self.pieces.get(piece)?;
        if !held.anchors().any(|anchor| anchor == point) {
            return None;
        }
        if let Some(label) = self.points.get(point).and_then(|p| p.label.clone()) {
            return Some(label);
        }
        self.automatic_label(piece, point)
    }

    /// The `P` name a point of `piece` falls back to when it carries no label.
    pub(crate) fn automatic_label(&self, piece: PieceKey, point: PointKey) -> Option<String> {
        let held = self.pieces.get(piece)?;
        if !held.anchors().any(|anchor| anchor == point) {
            return None;
        }
        let rank = held
            .anchors()
            .filter(|anchor| anchor.index() < point.index())
            .count();
        Some(format!("P{}", rank + 1))
    }

    /// The point of `piece` that shows `label`, if one does.
    pub fn shows_label(&self, piece: PieceKey, label: &str) -> Option<PointKey> {
        let held = self.pieces.get(piece)?;
        held.anchors()
            .find(|&point| self.label_of(piece, point).as_deref() == Some(label))
    }

    /// The piece that carries `name`, if one does.
    pub fn piece_named(&self, name: &str) -> Option<PieceKey> {
        self.pieces
            .iter()
            .find(|(_, piece)| piece.name == name)
            .map(|(key, _)| key)
    }

    /// The variable that carries `name`, if one does.
    pub fn variable_named(&self, name: &str) -> Option<VariableKey> {
        self.variables
            .iter()
            .find(|(_, variable)| variable.name == name)
            .map(|(key, _)| key)
    }

    /// The measure set that carries `name`, if one does.
    pub fn mannequin_named(&self, name: &str) -> Option<MannequinKey> {
        self.mannequins
            .iter()
            .find(|(_, set)| set.name == name)
            .map(|(key, _)| key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Binding, Winding};

    fn doc() -> (Doc, PieceKey, Vec<PointKey>) {
        let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
        let points: Vec<PointKey> = (0..3)
            .map(|i| doc.points.insert(Point::at(f64::from(i), 0.0)))
            .collect();
        let piece = doc
            .pieces
            .insert(Piece::polygon("Delantero", points.clone(), Winding::Cw));
        (doc, piece, points)
    }

    #[test]
    fn a_new_document_resolves_against_the_body_it_was_given() {
        let doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
        assert_eq!(doc.measures().map(|set| set.name.as_str()), Some("Etienne"));
        assert!(doc.pieces.is_empty());
    }

    #[test]
    fn a_new_document_carries_the_two_seam_tolerances() {
        let doc = Doc::new(MeasureSet::default());
        let tolerance = doc
            .variable_named("tolerancia_costura")
            .and_then(|key| doc.variables.get(key));
        assert_eq!(tolerance.map(|v| &v.value), Some(&Binding::Literal(0.5)));
        let ratio = doc
            .variable_named("tolerancia_ratio")
            .and_then(|key| doc.variables.get(key));
        assert_eq!(ratio.map(|v| &v.value), Some(&Binding::Literal(0.05)));
    }

    #[test]
    fn an_unlabelled_point_is_named_by_the_order_the_piece_gained_it() {
        let (doc, piece, points) = doc();
        assert_eq!(doc.label_of(piece, points[0]).as_deref(), Some("P1"));
        assert_eq!(doc.label_of(piece, points[2]).as_deref(), Some("P3"));
    }

    #[test]
    fn a_point_the_piece_does_not_run_through_has_no_name_in_it() {
        let (mut doc, piece, _) = doc();
        let stray = doc.points.insert(Point::at(9.0, 9.0));
        assert_eq!(doc.label_of(piece, stray), None);
        assert_eq!(doc.label_of(PieceKey::new(7, 0), stray), None);
    }

    #[test]
    fn a_label_its_author_wrote_wins_over_the_automatic_one() {
        let (mut doc, piece, points) = doc();
        doc.points
            .get_mut(points[1])
            .expect("the key is live")
            .label = Some("cadera_lat".to_owned());
        assert_eq!(
            doc.label_of(piece, points[1]).as_deref(),
            Some("cadera_lat")
        );
        assert_eq!(doc.shows_label(piece, "cadera_lat"), Some(points[1]));
        assert_eq!(doc.shows_label(piece, "P2"), None);
        assert_eq!(doc.shows_label(piece, "P3"), Some(points[2]));
    }

    #[test]
    fn a_point_is_found_by_every_piece_that_cites_it() {
        let (mut doc, piece, points) = doc();
        assert_eq!(doc.pieces_citing(points[0]), [piece]);
        let other = doc
            .pieces
            .insert(Piece::polygon("Trasero", points.clone(), Winding::Cw));
        assert_eq!(doc.pieces_citing(points[0]), [piece, other]);
        assert_eq!(doc.piece_named("Trasero"), Some(other));
        assert_eq!(doc.piece_named("Manga"), None);
    }
}
