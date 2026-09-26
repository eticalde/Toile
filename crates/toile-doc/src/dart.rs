use serde::{Deserialize, Serialize};

use crate::{
    ContourNode, Doc, DocError, EdgeRange, Identity, Piece, PieceKey, Point, PointKey, Seam,
    SeamKey, SeamOrientation, Segment,
};

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

/// The wedge a dart is declared over: a piece and three nodes it already has.
///
/// The counterpart of `DartWedge`, for the wedge somebody drew before there was
/// a dart to put on it. One piece for all three rather than one each, so a
/// declaration cannot name three nodes of three contours: a node of another
/// piece is simply a node this contour does not have.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawnWedge {
    /// The piece whose contour already runs through the three nodes.
    pub piece: PieceKey,
    /// First leg, apex and second leg, in contour order.
    pub nodes: [PointKey; 3],
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

impl Dart {
    /// The seam that shuts a wedge, whoever is putting one there.
    ///
    /// Opposed, because the two sides meet at the apex: walked in contour order
    /// the first runs leg to apex and the second apex to leg, so the pairing
    /// that sews the wedge shut is head to tail. Written once here and read
    /// back by `is_shut_by`, so that what an edit sews and what a reader will
    /// accept cannot come apart.
    pub fn shutting_seam(piece: PieceKey, apex: PointKey, legs: (PointKey, PointKey)) -> Seam {
        Seam::plain(
            EdgeRange::between(piece, legs.0, apex),
            EdgeRange::between(piece, apex, legs.1),
            SeamOrientation::Opposed,
        )
    }

    /// Whether `sewn` is still the seam that shuts this wedge on `piece`.
    ///
    /// The two sides and the way round, and nothing else: what a seam expects
    /// of the two lengths is its author's to write, and a dart eased shut is
    /// shut. Turned the other way round it is not — that pairing sews each leg
    /// to the apex and leaves the wedge open with a record that says dart.
    pub(crate) fn is_shut_by(&self, piece: PieceKey, sewn: &Seam) -> bool {
        let shutting = Dart::shutting_seam(piece, self.apex, self.legs);
        sewn.a == shutting.a && sewn.b == shutting.b && sewn.orientation == shutting.orientation
    }
}

/// Whether any dart of the document is written over `point`.
///
/// One rule for every edit that moves a contour: a dart's record names three
/// nodes standing together, `json::check` refuses a document where they do
/// not, and an editor that allowed the edit anyway would let a person save a
/// product they can never open. It is the argument `remove_seam` already
/// makes for a dart's thread, applied to the three nodes it is sewn through.
pub(crate) fn over(doc: &Doc, point: PointKey) -> bool {
    doc.darts
        .iter()
        .any(|(_, dart)| dart.apex == point || dart.legs.0 == point || dart.legs.1 == point)
}

/// Whether a node seated at `seat` of `piece` would land inside a wedge.
///
/// Inside and not merely beside: a node taking the seat after a wedge's first
/// or second node splits the three apart, while one after its last leg falls
/// clear of them.
pub(crate) fn splits(doc: &Doc, piece: PieceKey, seat: usize) -> bool {
    let Some(held) = doc.pieces.get(piece) else {
        return false;
    };
    doc.darts.iter().any(|(_, dart)| {
        let head = held.node_index(dart.legs.0);
        head.is_some_and(|head| seat == head + 1 || seat == head + 2)
    })
}

/// Where a wedge's three nodes stand in a contour, if they stand together.
///
/// The seat of the first leg, with the apex and the second leg at the two seats
/// after it. The one place a wedge is held against the contour it describes —
/// asked by the edit that declares a dart, by the one that takes a cut one back
/// out, and by every file that is opened — so that the three cannot drift apart
/// on what a wedge is.
///
/// Two of the three being one node lands on the same refusal, and for the same
/// reason: no node follows itself.
pub(crate) fn wedge_seat(held: &Piece, nodes: [PointKey; 3]) -> Result<usize, DocError> {
    let mut seats = [0usize; 3];
    for (seat, point) in seats.iter_mut().zip(nodes) {
        *seat = held.node_index(point).ok_or(DocError::NoSuchNode)?;
    }
    if seats[1] != seats[0] + 1 || seats[2] != seats[1] + 1 {
        return Err(DocError::ScatteredWedge);
    }
    Ok(seats[0])
}

impl DrawnWedge {
    /// The three nodes as a dart names them: the two legs and the apex.
    pub fn legs(&self) -> (PointKey, PointKey) {
        (self.nodes[0], self.nodes[2])
    }

    /// The node the wedge closes onto.
    pub fn apex(&self) -> PointKey {
        self.nodes[1]
    }
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
    use crate::{SeamKind, Winding};

    fn wedge() -> (PieceKey, PointKey, (PointKey, PointKey), Dart) {
        let piece = PieceKey::new(0, 0);
        let apex = PointKey::new(2, 0);
        let legs = (PointKey::new(1, 0), PointKey::new(3, 0));
        let dart = Dart {
            apex,
            legs,
            seam: SeamKey::new(0, 0),
            fold: FoldDirection::TowardEnd,
        };
        (piece, apex, legs, dart)
    }

    /// One pairing shuts a wedge and the rest leave it open, whatever the seam
    /// expects of the two lengths it joins.
    #[test]
    fn only_the_pairing_that_meets_at_the_apex_shuts_a_wedge() {
        let (piece, apex, legs, dart) = wedge();
        let shutting = Dart::shutting_seam(piece, apex, legs);
        assert_eq!(shutting.orientation, SeamOrientation::Opposed);
        assert!(dart.is_shut_by(piece, &shutting));

        let eased = Seam {
            kind: SeamKind::Eased { expected_cm: 0.5 },
            ..shutting
        };
        assert!(dart.is_shut_by(piece, &eased), "a dart eased shut is shut");
        let turned = Seam {
            orientation: SeamOrientation::Aligned,
            ..shutting
        };
        assert!(!dart.is_shut_by(piece, &turned), "each leg to the apex");
        let elsewhere = Dart::shutting_seam(PieceKey::new(1, 0), apex, legs);
        assert!(!dart.is_shut_by(piece, &elsewhere), "another piece's cloth");
    }

    /// A wedge is three seats in a row or it is no wedge, and that includes the
    /// three written out of order.
    #[test]
    fn a_wedge_stands_together_in_the_contour_or_it_does_not_stand() {
        let keys: Vec<PointKey> = (0..5).map(|index| PointKey::new(index, 0)).collect();
        let piece = Piece::polygon("Trasero", keys.clone(), Winding::Cw);
        assert_eq!(wedge_seat(&piece, [keys[1], keys[2], keys[3]]), Ok(1));
        for asking in [
            [keys[3], keys[2], keys[1]],
            [keys[0], keys[1], keys[3]],
            [keys[1], keys[1], keys[2]],
            // The closure is implicit, and a wedge that straddles it is refused
            // here as it is unreachable by a cut: the three seats are counted
            // forward from the first leg and never wrapped.
            [keys[4], keys[0], keys[1]],
        ] {
            assert_eq!(
                wedge_seat(&piece, asking),
                Err(DocError::ScatteredWedge),
                "{asking:?}"
            );
        }
        let stray = [keys[1], keys[2], PointKey::new(9, 0)];
        assert_eq!(wedge_seat(&piece, stray), Err(DocError::NoSuchNode));
    }

    #[test]
    fn a_wedge_node_becomes_a_contour_node_once_it_has_a_key() {
        let wedge = WedgeNode::line(Identity::New, Point::at(0.0, 0.0));
        let node = wedge.node(PointKey::new(4, 0));
        assert_eq!(node.point, PointKey::new(4, 0));
        assert_eq!(node.segment, Segment::Line);
        assert_eq!(node.samples, 1);
    }
}
