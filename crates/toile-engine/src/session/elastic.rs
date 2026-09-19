use toile_doc::Elastic;

use super::place::Band;
use super::{Session, ShapePipeline};
use crate::couture::{self, Held};
use crate::draft::Draft;

/// What the document's elastics come to on the cloth now on the stand.
pub(super) struct Elastics {
    /// The edges they hold, addressed in the combined constraint set.
    pub(super) held: Vec<Held>,
    /// The ring of cloth they hold the product by, when they hold one.
    pub(super) band: Option<Band>,
}

/// One elastic read onto one piece's mesh: the boundary vertices it runs over
/// and where it opens in the constraint set.
struct Run {
    at: usize,
    base: usize,
    verts: Vec<u32>,
}

impl Session {
    /// Every elastic of the document, read onto the meshes now on the stand.
    ///
    /// Read afresh rather than remembered, for the reason the sewing is: a
    /// shape edit moves an anchor's fraction along its own piece and a rebuild
    /// hands that piece a whole new set of vertices, so a reading is only ever
    /// true of the meshes it was taken against. That is why the two moments
    /// rest lengths are compiled — a derive and an install — both come through
    /// here, and why an elastic cannot quietly vanish on the first drag.
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
            let Some(run) = self.run_of(draft, &pipes, &bases, elastic) else {
                continue;
            };
            let pipe = pipes[run.at];
            let ratio = elastic.ratio as f32;
            let compliance = couture::compliance_of(elastic.strength);
            for pair in run.verts.windows(2) {
                if let Some(edge) = pipe.edge_index(pair[0], pair[1]) {
                    held.push(Held {
                        edge: run.base + edge,
                        ratio,
                        compliance,
                    });
                }
            }
            widen(&mut waists, run.at, pipe, &run.verts);
            for &v in &run.verts {
                ordinate += pipe.pos2d[v as usize][1];
                counted += 1;
            }
        }
        let band = (counted > 0).then(|| Band {
            girth: waists.iter().map(|&(_, lo, hi)| hi - lo).sum(),
            at: ordinate / counted as f64,
        });
        Elastics { held, band }
    }

    /// One elastic on the cloth: which piece it lands on, where that piece's
    /// edges begin, and the boundary vertices the stretch runs over.
    ///
    /// `None` for an elastic the cloth cannot answer for — a piece too partial
    /// to mesh, an anchor on a node its piece no longer runs through, or a
    /// stretch with no length. Left out of the solve rather than guessed at:
    /// an elastic read onto the wrong vertices would hold the garment in by
    /// somewhere nobody asked for.
    fn run_of(
        &self,
        draft: &Draft,
        pipes: &[&ShapePipeline],
        bases: &[usize],
        elastic: &Elastic,
    ) -> Option<Run> {
        let at = self.index_of(elastic.at.piece()?)?;
        let head = draft.anchor_fraction(&elastic.at.head)?;
        let tail = draft.anchor_fraction(&elastic.at.tail)?;
        // The walk the way the contour runs, as a seam's side is measured, so
        // a stretch that passes the closure is as long as the walk.
        let span = (tail - head).rem_euclid(1.0);
        if span <= f64::EPSILON {
            return None;
        }
        let verts = pipes[at].boundary_run((head, span));
        (verts.len() >= 2).then(|| Run {
            at,
            base: bases[at],
            verts,
        })
    }
}

/// Where each piece's edges begin in the product's combined constraints.
///
/// The constraints are concatenated in the order the pieces stand, so a
/// piece's edges begin past every edge of every piece before it — the same
/// arithmetic [`couture::offsets`] does over vertices, counted over edges
/// because that is what an elastic addresses.
fn edge_bases(pipes: &[&ShapePipeline]) -> Vec<usize> {
    let mut at = 0;
    let mut bases = Vec::with_capacity(pipes.len());
    for pipe in pipes {
        bases.push(at);
        at += pipe.edges.len();
    }
    bases
}

/// Widens one piece's entry to take in the cloth a stretch covers.
///
/// Per piece and not per elastic, so two elastics laid over one stretch — the
/// document allows them, as it allows two seams — count the cloth they share
/// once rather than twice.
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
    use toile_doc::{Command, Doc, EdgeRange, Identity, MeasureSet, Piece, Point, Winding};

    use super::*;
    use crate::body::Collider;
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
        assert!(!read.held.is_empty(), "the hem carries edges");
        for one in &read.held {
            let (a, b) = pipe.edges[one.edge];
            for v in [a, b] {
                let p = pipe.pos2d[v as usize];
                assert!(
                    p[1].abs() < 1.0e-9,
                    "edge {} leaves the hem at {p:?}",
                    one.edge
                );
            }
            assert!((one.ratio - 0.85).abs() < 1.0e-7);
        }
        // Within one boundary sample of the hem's own length: a stretch covers
        // the vertices the mesher laid, and the last one before a corner sits
        // a little short of it.
        let band = read.band.expect("the elastic holds the panel by its hem");
        assert!((band.girth - 0.20).abs() < 0.01, "{} of cloth", band.girth);
        assert!(band.at.abs() < 1.0e-9, "along the hem");
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
}
