use toile_sim::xpbd::Hung;

use super::Session;
use super::run::edge_bases;
use crate::body::{Collider, at_station};
use crate::couture::{hung_at, offsets};

mod heading;
mod station;

impl Session {
    /// Every hang of the document, read onto the meshes now on the stand and
    /// onto the rings of the body the drape falls on.
    ///
    /// Read afresh rather than remembered, for the reason the elastics are: a
    /// reading is only ever true of the meshes and the body it was taken
    /// against. That is why every message that can move either — a derive, a
    /// rebuild, a body re-solved — carries a fresh one.
    ///
    /// One hang can hold more than one stretch, for the reason one elastic
    /// can: a range written on a piece drawn against a fold is on the cut
    /// piece twice, once on each side of the crease. A vertex more than one of
    /// them names is held by the first, because it can only be pulled once.
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
        // Across every hang and not within one, the way `couture::hold` counts
        // an edge two elastics cover: this is where all of them can be seen.
        let mut already: Vec<u32> = Vec::new();
        for (_, hang) in draft.doc().hangs.iter() {
            // A body with no rings — the demo ball, the cube a test bakes —
            // measures no height, and cloth held at a height nobody measured is
            // worse off than cloth held at none.
            let Some(belt) = at_station(collider.belts(), &hang.station) else {
                continue;
            };
            let mut at: Vec<u32> = Vec::new();
            for run in self.runs_of(draft, &pipes, &bases, hang.at) {
                let base = vertex_bases[run.at];
                // One pull a substep per vertex, whatever names it: the solver
                // clamps each pull on its own, so a vertex two runs name moves
                // twice the step the body's own field allows in one. The two
                // stretches of a folded piece meet on the crease, and two hangs
                // meet wherever the tracts they are written on do.
                for v in run.verts.iter().map(|&v| v + base) {
                    if !already.contains(&v) {
                        already.push(v);
                        at.push(v);
                    }
                }
            }
            if !at.is_empty() {
                hung.push(hung_at(at, belt.height));
            }
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
    use crate::body::{Phenotype, body_mesh};
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

    /// A product declares its hung lines topmost first, and nothing at all when
    /// nobody hung it.
    ///
    /// Which line is read first matters because it is the one stood on a ring,
    /// and the two candidates here are a hand's breadth apart on the pattern.
    /// The document holds the lower one second, so a reader that took the first
    /// would pass this test by accident — the hems are written in the order
    /// that catches it. Both come back, because a garment hung from two
    /// stations goes round two rings.
    #[test]
    fn a_product_declares_the_highest_of_its_hung_lines() {
        let (mut doc, piece) = panel();
        let at = |l| doc.shows_label(piece, l).expect("the panel names it");
        let (top, side) = (
            EdgeRange::between(piece, at("a"), at("b")),
            EdgeRange::between(piece, at("b"), at("c")),
        );
        for (run, station) in [(top, Hang::WAIST), (side, "cadera")] {
            Command::AddHang {
                identity: Identity::New,
                hang: Hang::new(run, station),
            }
            .apply(&mut doc)
            .expect("both ends are nodes of the panel");
        }
        let bare = Session::from_doc(panel().0, Collider::demo()).expect("the panel drapes");
        assert!(bare.declared().is_empty(), "nobody hung it");

        let session = Session::from_doc(doc, Collider::demo()).expect("the panel drapes");
        let worn = session.declared();
        let names: Vec<&str> = worn.iter().map(|one| one.station.as_str()).collect();
        assert_eq!(names, [Hang::WAIST, "cadera"], "topmost first");
        // The pattern's y runs upward in metres, so the drawn top edge is the
        // ordinate zero the mesher hands back and the side runs down from it.
        assert!(worn[0].at.abs() < 1.0e-9, "the top edge, at {}", worn[0].at);
        // One piece carries both, so both rings name it: which of them it goes
        // on is the grouping's question and not this reading's.
        assert_eq!(worn[0].on, [0]);
        assert_eq!(worn[1].on, [0]);
    }

    /// Two hangs that meet on a node hold the vertex there once.
    ///
    /// The solver clamps each pull on its own, so a vertex two runs name is
    /// carried twice the step in one substep — and the two tracts leaving one
    /// node are two presses apart in the studio, not a document nobody would
    /// write. The rule is the elastics' own: what covers one place twice holds
    /// it once, and the place to say so is where every hang can be seen.
    #[test]
    #[ignore = "release-only: a real body baked, for the rings a station names"]
    fn two_hangs_that_meet_on_a_node_hold_the_vertex_there_once() {
        let (mut doc, piece) = panel();
        let at = |l| doc.shows_label(piece, l).expect("the panel names it");
        let (hem, side) = (
            EdgeRange::between(piece, at("a"), at("b")),
            EdgeRange::between(piece, at("c"), at("a")),
        );
        for run in [hem, side] {
            Command::AddHang {
                identity: Identity::New,
                hang: Hang::new(run, Hang::WAIST),
            }
            .apply(&mut doc)
            .expect("both ends are nodes of the panel");
        }
        let mesh = body_mesh(&Phenotype::default(), &[0.0; 20]);
        let body = Collider::bake(&mesh).expect("the Anny body is closed and orientable");
        let session = Session::from_doc(doc, body).expect("the panel drapes");
        let mut once: Vec<u32> = session.hung_cloth().iter().map(|&(v, _)| v).collect();
        let named = once.len();
        once.sort_unstable();
        once.dedup();
        assert!(named > 2, "both tracts hang: {named} vertices");
        assert_eq!(once.len(), named, "node `a` is held by one pull, not two");
    }
}
