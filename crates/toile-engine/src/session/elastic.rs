use super::place::Band;
use super::run::edge_bases;
use super::{Session, ShapePipeline};
use crate::couture::{self, Held};

/// What the document's elastics come to on the cloth now on the stand.
pub(super) struct Elastics {
    /// The edges they hold, addressed in the combined constraint set.
    pub(super) held: Vec<Held>,
    /// The ring of cloth they hold the product by, when they hold one.
    pub(super) band: Option<Band>,
}

impl Session {
    /// Every elastic of the document, read onto the meshes now on the stand.
    ///
    /// Read afresh rather than remembered, for the reason [`Session::runs_of`]
    /// gives. That is why the two moments rest lengths are compiled — a derive
    /// and an install — both come through here, and why an elastic cannot
    /// quietly vanish on the first drag.
    ///
    /// One elastic can come to more than one stretch of cloth: on a piece
    /// drawn against a fold it holds the drawn half and its mirror, which are
    /// one hem on the cut piece and two runs of boundary on the mesh.
    pub(super) fn elastics(&self) -> Elastics {
        let mut held = Vec::new();
        let mut waists: Vec<(usize, f64, f64)> = Vec::new();
        let (mut ordinate, mut counted) = (0.0f64, 0usize);
        let Some(draft) = self.draft.as_ref() else {
            return Elastics { held, band: None };
        };
        let pipes = self.pipelines();
        let bases = edge_bases(&pipes);
        for (_, elastic) in draft.doc().elastics.iter() {
            for run in self.runs_of(draft, &pipes, &bases, elastic.at) {
                let pipe = pipes[run.at];
                let edges: Vec<usize> = run
                    .verts
                    .windows(2)
                    .filter_map(|pair| pipe.edge_index(pair[0], pair[1]))
                    .map(|edge| run.base + edge)
                    .collect();
                held.push(Held {
                    edges,
                    ratio: elastic.ratio as f32,
                    compliance: couture::compliance_of(elastic.strength),
                });
                widen(&mut waists, run.at, pipe, &run.verts);
                for &v in &run.verts {
                    ordinate += pipe.pos2d[v as usize][1];
                    counted += 1;
                }
            }
        }
        let band = (counted > 0).then(|| Band {
            girth: waists.iter().map(|&(_, lo, hi)| hi - lo).sum(),
            at: ordinate / counted as f64,
        });
        Elastics { held, band }
    }

    /// Every stretch of cloth the product's elastics hold, as pairs of
    /// vertices of the combined state with the length each is held to.
    ///
    /// Plain indices and plain metres, for the reason [`Session::sewn_pairs`]
    /// hands seams over that way: a test measuring what a band came to on the
    /// body, or a client drawing one, does it without the solver's vocabulary
    /// crossing over. Summed in three dimensions and set against these rest
    /// lengths, it is the one reading that tells an elastic holding its ratio
    /// from one being dragged open.
    pub fn held_cloth(&self) -> Vec<((u32, u32), f32)> {
        let cons = self.constraints();
        cons.held
            .iter()
            .map(|&e| {
                let at = e as usize;
                ((cons.a[at], cons.b[at]), cons.rest[at])
            })
            .collect()
    }
}

/// Widens one piece's entry to take in the cloth a stretch covers.
///
/// Per piece and not per elastic: the cloth two elastics share is one length
/// of cloth, counted once here and held once by [`couture::hold`], which is
/// where what two elastics over one stretch mean is written.
///
/// It is what makes a folded piece come out whole as well. The drawn half and
/// its mirror are two stretches that meet on the crease, so one span taken
/// across both of them is the cloth the band goes round, while adding their
/// two widths would count the crease twice over.
fn widen(waists: &mut Vec<(usize, f64, f64)>, at: usize, pipe: &ShapePipeline, verts: &[u32]) {
    let (lo, hi) = verts.iter().fold((f64::MAX, f64::MIN), |(lo, hi), &v| {
        let x = pipe.pos2d[v as usize][0];
        (lo.min(x), hi.max(x))
    });
    match waists.iter_mut().find(|held| held.0 == at) {
        Some(held) => {
            held.1 = held.1.min(lo);
            held.2 = held.2.max(hi);
        }
        None => waists.push((at, lo, hi)),
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "an edge nothing held carries the very bits it was given"
    )]

    use toile_doc::{
        Command, Doc, EdgeRange, Identity, MeasureSet, Piece, Point, Symmetry, Winding,
    };

    use super::*;
    use crate::body::Collider;
    use crate::couture::COMPLIANCE;
    use crate::draft::PieceKey;

    /// A panel two tenths of a metre wide, with its hem named so a stretch can
    /// be written over it.
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

    /// The hem read onto the cloth: the edges under it, and the cloth they
    /// cover.
    #[test]
    fn an_elastic_lands_on_the_boundary_edges_of_the_stretch_it_names() {
        let (mut doc, piece) = panel();
        let named = |doc: &Doc, l| doc.shows_label(piece, l).expect("the panel names it");
        let at = EdgeRange::between(piece, named(&doc, "a"), named(&doc, "b"));
        Command::AddElastic {
            identity: Identity::New,
            elastic: toile_doc::Elastic::new(at, 0.85, 10.0),
        }
        .apply(&mut doc)
        .expect("both ends are nodes of the panel");

        let session = Session::from_doc(doc, Collider::demo()).expect("the panel drapes");
        let read = session.elastics();
        let pipe = session.pipelines()[0];
        let [one] = &read.held[..] else {
            panic!("one elastic, one stretch: {:?}", read.held);
        };
        assert!(!one.edges.is_empty(), "the hem carries edges");
        for &edge in &one.edges {
            let (a, b) = pipe.edges[edge];
            for v in [a, b] {
                let p = pipe.pos2d[v as usize];
                assert!(p[1].abs() < 1.0e-9, "edge {edge} leaves the hem at {p:?}");
            }
        }
        assert!((one.ratio - 0.85).abs() < 1.0e-7);
        // Within one boundary sample of the hem's own length: a stretch covers
        // the vertices the mesher laid, and the last one before a corner sits
        // a little short of it.
        let band = read.band.expect("the elastic holds the panel by its hem");
        assert!((band.girth - 0.20).abs() < 0.01, "{} of cloth", band.girth);
        assert!(band.at.abs() < 1.0e-9, "along the hem");
    }

    /// The same hem on a panel drawn to a fold is two stretches, and the band
    /// they make is the whole cloth's.
    ///
    /// The number is what a placement passes to the body as the ring to match:
    /// a band reading half its girth sends a waistband off to look for a ring
    /// half the size, and on a measured body there is one.
    #[test]
    fn an_elastic_on_a_panel_at_the_fold_makes_a_band_of_the_whole_cloth() {
        let (mut doc, piece) = panel();
        let named = |doc: &Doc, l| doc.shows_label(piece, l).expect("the panel names it");
        let at = EdgeRange::between(piece, named(&doc, "a"), named(&doc, "b"));
        let axis = EdgeRange::between(piece, named(&doc, "b"), named(&doc, "c"));
        Command::AddSymmetry {
            identity: Identity::New,
            symmetry: Symmetry::fold(axis),
        }
        .apply(&mut doc)
        .expect("both ends are nodes of the panel");
        Command::AddElastic {
            identity: Identity::New,
            elastic: toile_doc::Elastic::new(at, 0.85, 10.0),
        }
        .apply(&mut doc)
        .expect("both ends are nodes of the panel");

        let session = Session::from_doc(doc, Collider::demo()).expect("the panel drapes");
        let read = session.elastics();
        assert_eq!(
            read.held.len(),
            2,
            "the hem and its mirror: {:?}",
            read.held
        );
        let band = read.band.expect("the elastic holds the panel by its hem");
        // One span across both stretches and not the two widths added: they
        // meet on the crease, which the sum would count twice.
        assert!((band.girth - 0.40).abs() < 0.01, "{} of cloth", band.girth);
        assert!(
            band.at.abs() < 1.0e-9,
            "the mirror keeps the hem's own line"
        );
    }

    /// And a product with no elastic reads as one: nothing held, and nowhere
    /// it hangs from, so the placement is left exactly where it was.
    #[test]
    fn a_product_with_no_elastic_holds_nothing_and_hangs_by_nothing() {
        let (doc, _) = panel();
        let session = Session::from_doc(doc, Collider::demo()).expect("the panel drapes");
        let read = session.elastics();
        assert!(read.held.is_empty());
        assert_eq!(read.band, None);
    }

    /// The set the solver is handed carries what the document holds: shorter
    /// rests under the band, its compliance on those edges, and every other
    /// edge of the product exactly as the concatenation left it.
    ///
    /// This is what tells [`Session::constraints`] from the bare
    /// concatenation. Every other reading of a band goes through the
    /// placement, which never touches `hold`, so a session that quietly handed
    /// the plain constraints out would drape a garment with a waistband drawn
    /// on it and no waistband in the solver.
    #[test]
    fn what_the_solver_is_handed_carries_what_the_elastics_hold() {
        let (mut doc, piece) = panel();
        let named = |doc: &Doc, l| doc.shows_label(piece, l).expect("the panel names it");
        let at = EdgeRange::between(piece, named(&doc, "a"), named(&doc, "b"));
        Command::AddElastic {
            identity: Identity::New,
            elastic: toile_doc::Elastic::new(at, 0.85, couture::HOLDS_ITS_RATIO),
        }
        .apply(&mut doc)
        .expect("both ends are nodes of the panel");

        let session = Session::from_doc(doc, Collider::demo()).expect("the panel drapes");
        let plain = couture::combine_constraints(&session.pipelines(), COMPLIANCE);
        let handed = session.constraints();
        let held = session.elastics().held;
        let edges: Vec<usize> = held.iter().flat_map(|one| one.edges.clone()).collect();
        assert!(!edges.is_empty(), "the hem carries edges");
        assert_ne!(handed.rest, plain.rest, "the band reached the solver");
        for one in &held {
            for &edge in &one.edges {
                assert_eq!(handed.rest[edge], plain.rest[edge] * one.ratio);
                assert_eq!(handed.compliance[edge], one.compliance);
            }
        }
        for edge in (0..plain.len()).filter(|e| !edges.contains(e)) {
            assert_eq!(handed.rest[edge], plain.rest[edge]);
            assert_eq!(handed.compliance[edge], plain.compliance[edge]);
        }
        // And those edges are the ones swept again: a band the cloth outvotes
        // holds nothing, whatever its rest length says.
        let mut named = handed.held.clone();
        named.sort_unstable();
        let mut want: Vec<u32> = edges.iter().map(|&e| e as u32).collect();
        want.sort_unstable();
        assert_eq!(named, want);
        assert!(handed.held_passes > 0);
    }

    /// And a product with no elastic is handed the concatenation itself, bit
    /// for bit — the reduction the drape goldens stand on, asked of the
    /// session rather than of `hold` alone.
    #[test]
    fn what_a_product_with_no_elastic_is_handed_is_the_concatenation() {
        let (doc, _) = panel();
        let session = Session::from_doc(doc, Collider::demo()).expect("the panel drapes");
        let plain = couture::combine_constraints(&session.pipelines(), COMPLIANCE);
        let handed = session.constraints();
        assert_eq!(handed.rest, plain.rest);
        assert_eq!(handed.compliance, plain.compliance);
        assert!(handed.held.is_empty(), "and nothing is swept again");
    }
}
