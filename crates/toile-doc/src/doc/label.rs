use crate::{Doc, PieceKey, PointKey};

/// What a piece calls one of its points.
impl Doc {
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
}
