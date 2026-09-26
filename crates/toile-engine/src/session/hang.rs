use toile_sim::xpbd::Hung;

use super::Session;
use super::run::edge_bases;
use crate::body::{Collider, at_station};
use crate::couture::{hung_at, offsets};

impl Session {
    /// Every hang of the document, read onto the meshes now on the stand and
    /// onto the rings of the body the drape falls on.
    ///
    /// Read afresh rather than remembered, for the reason the elastics are: a
    /// reading is only ever true of the meshes and the body it was taken
    /// against. That is why every message that can move either — a derive, a
    /// rebuild, a body re-solved — carries a fresh one.
    pub(super) fn hung(&self) -> Vec<Hung> {
        self.hung_on(&self.collider)
    }

    /// The same, against a body that is not on the stand yet.
    ///
    /// The body is the argument because the height comes from it: reading the
    /// session's own while a new one is on its way would hold the garment at
    /// the waist of the person before it.
    pub(super) fn hung_on(&self, collider: &Collider) -> Vec<Hung> {
        let Some(draft) = self.draft.as_ref() else {
            return Vec::new();
        };
        // Before anything is allocated or walked, so a product hung from
        // nothing reaches the solver with the empty slice it has always had.
        if draft.doc().hangs.is_empty() {
            return Vec::new();
        }
        let pipes = self.pipelines();
        let bases = edge_bases(&pipes);
        let vertex_bases = offsets(&pipes);
        let mut hung = Vec::new();
        for (_, hang) in draft.doc().hangs.iter() {
            // A body with no rings — the demo ball, the cube a test bakes —
            // measures no height, and cloth held at a height nobody measured is
            // worse off than cloth held at none.
            let Some(belt) = at_station(collider.belts(), &hang.station) else {
                continue;
            };
            let Some(run) = self.run_of(draft, &pipes, &bases, hang.at) else {
                continue;
            };
            let base = vertex_bases[run.at];
            let at = run.verts.iter().map(|&v| v + base).collect();
            hung.push(hung_at(at, belt.height));
        }
        hung
    }

    /// Every vertex of the combined state the document's hangs hold, with the
    /// height each is held at, in metres.
    ///
    /// Plain indices and plain metres, for the reason [`Session::held_cloth`]
    /// hands the elastics over that way: a test asking whether the garment
    /// stayed where it was put reads it without the solver's vocabulary
    /// crossing over.
    pub fn hung_cloth(&self) -> Vec<(u32, f32)> {
        self.hung()
            .iter()
            .flat_map(|run| run.at.iter().map(|&v| (v, run.height)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use toile_doc::{Command, Doc, EdgeRange, Hang, Identity, MeasureSet, Piece, Point, Winding};

    use super::*;
    use crate::draft::PieceKey;

    /// A panel with its hem named, so a run can be written over it.
    fn panel() -> (Doc, PieceKey) {
        let mut doc = Doc::new(MeasureSet::new("Maniquí", []));
        let corners = [("a", 0.0, 0.0), ("b", 20.0, 0.0), ("c", 20.0, 15.0)];
        let points = corners
            .iter()
            .map(|&(label, x, y)| doc.points.insert(Point::at(x, y).named(label)))
            .collect::<Vec<_>>();
        let piece = doc
            .pieces
            .insert(Piece::polygon("Panel", points, Winding::Cw));
        (doc, piece)
    }

    /// The reduction every drape golden stands on, asked of the session: a
    /// product nobody hung reaches the solver holding nothing at all.
    #[test]
    fn a_product_nobody_hung_is_held_at_no_height() {
        let (doc, _) = panel();
        let session = Session::from_doc(doc, Collider::demo()).expect("the panel drapes");
        assert!(session.hung().is_empty());
        assert!(session.hung_cloth().is_empty());
    }

    /// And a body that measures no rings holds nothing either, however the
    /// document is written. The demo ball is such a body, and a height nobody
    /// measured is worse than no height: the garment would be pulled to a
    /// number this code invented.
    #[test]
    fn a_body_with_no_rings_holds_the_garment_at_no_height() {
        let (mut doc, piece) = panel();
        let named = |doc: &Doc, l| doc.shows_label(piece, l).expect("the panel names it");
        let at = EdgeRange::between(piece, named(&doc, "a"), named(&doc, "b"));
        Command::AddHang {
            identity: Identity::New,
            hang: Hang::new(at, Hang::WAIST),
        }
        .apply(&mut doc)
        .expect("both ends are nodes of the panel");

        let ball = Collider::demo();
        assert!(ball.belts().is_empty(), "the demo body measures no rings");
        let session = Session::from_doc(doc, ball).expect("the panel drapes");
        assert!(!session.draft().expect("a document").doc().hangs.is_empty());
        assert!(session.hung().is_empty(), "and nothing is held");
    }
}
