use toile_sim::xpbd::Seams;

use super::place::Placement;
use super::seam::{SeamFault, pair_seam_anchored};
use super::{Elsewhere, Loose, Session, place};
use crate::couture::{self, SEAM_PASSES, SEAM_STEP, ShapePipeline};
use crate::draft::{PieceKey, SeamKey};

/// One document seam, paired onto the combined solver state.
pub(super) struct Sewn {
    /// Where each side's piece stands among the ones on the stand.
    pub(super) sides: [usize; 2],
    /// Mean pattern abscissa of the sewn stretch on each side.
    ///
    /// Where across its own piece the seam runs, which is the whole of what
    /// the placement reads: two seams on one piece say how wide a turn that
    /// piece takes around the body, and which way round it takes it.
    pub(super) at: [f64; 2],
    /// The abscissae the sewn stretch begins and ends at on each side, in
    /// pairing order.
    ///
    /// What carries a declaration across a seam. Two cloths sewn together are
    /// one cloth, so the two ends of the stretch name the same two places of
    /// the garment on either side, and a line declared on one piece has a line
    /// of the other it is: see [`super::place::carried`].
    pub(super) ends: [[f64; 2]; 2],
    /// The paired vertices, as indices into the combined state.
    pub(super) a: Vec<u32>,
    pub(super) b: Vec<u32>,
    /// Whether this is the seam that shuts a dart.
    ///
    /// The solver runs it like any other: a dart closes because its two legs
    /// are sewn. The placement must not see it at all — it reads the seams to
    /// chain the pieces into a strip, and a seam with one piece on both sides
    /// chains nothing while looking exactly like the case that rule refuses.
    pub(super) dart: bool,
}

impl Session {
    /// Every document seam paired onto the meshes now on the stand, and every
    /// seam that could not be.
    ///
    /// Read afresh rather than remembered. A shape edit moves the anchors'
    /// fractions along their own piece, and a rebuild hands a piece a whole
    /// new set of vertices, so a pairing is only ever true of the meshes it
    /// was taken against.
    pub(super) fn sewn(&self) -> (Vec<Sewn>, Vec<(SeamKey, SeamFault)>) {
        let (mut sewn, mut faults) = (Vec::new(), Vec::new());
        let Some(draft) = self.draft.as_ref() else {
            return (sewn, faults);
        };
        let pipes = self.pipelines();
        let bases = couture::offsets(&pipes);
        let held: Vec<Option<PieceKey>> = self.draping.iter().map(|d| d.piece).collect();
        let stands = |piece| held.iter().position(|&k| k == Some(piece));
        for (key, seam) in draft.doc().seams.iter() {
            let (Some(pa), Some(pb)) = (seam.a.piece(), seam.b.piece()) else {
                continue;
            };
            // A seam onto a piece that is not draping is not a fault: a piece
            // too partial to mesh is a drawing in progress, and the seam waits
            // for it rather than complaining about it.
            let (Some(ia), Some(ib)) = (stands(pa), stands(pb)) else {
                continue;
            };
            match pair_seam_anchored(draft, seam, pipes[ia], bases[ia], pipes[ib], bases[ib]) {
                Ok((a, b)) => sewn.push(Sewn {
                    dart: draft.doc().darts.iter().any(|(_, it)| it.seam == key),
                    sides: [ia, ib],
                    at: [
                        mean_at(pipes[ia], &a, bases[ia]),
                        mean_at(pipes[ib], &b, bases[ib]),
                    ],
                    ends: [
                        ends_at(pipes[ia], &a, bases[ia]),
                        ends_at(pipes[ib], &b, bases[ib]),
                    ],
                    a,
                    b,
                }),
                Err(why) => faults.push((key, why)),
            }
        }
        (sewn, faults)
    }

    /// Pairs the seams again, keeps whatever refused to pair, and hands back
    /// the set the solver is to run on.
    pub(super) fn resew(&mut self) -> Seams {
        let (sewn, faults) = self.sewn();
        self.faults = faults;
        stitches(&sewn)
    }

    /// Where the body of the product is let go, when the seams place it.
    ///
    /// The ring itself and not only its effect: a client showing where a
    /// garment was put, and a test measuring it against the body, both need
    /// the centre, the size and the height that were chosen.
    ///
    /// The ring carrying the most pieces, which is the body of the garment. A
    /// shirt goes round two of them and its collar is the other;
    /// [`Session::rings`] is all of them.
    pub fn layout(&self) -> Option<couture::Layout> {
        self.rings().into_iter().next()
    }

    /// Every ring the product goes round, the body of the garment first.
    ///
    /// One for most garments and one per declared station for the rest: a
    /// shirt's body is hung from the chest and its collar from the neck, and
    /// those are two heights and two girths. Empty when nothing is placed.
    pub fn rings(&self) -> Vec<couture::Layout> {
        self.placed().rings
    }

    /// The same release, with what the body had to say about the station the
    /// document declared and what it could not place at all.
    ///
    /// One call and not three, because all of it comes out of the one choice of
    /// rings: asking separately would place the product twice and let the
    /// readings disagree about a garment nobody moved in between.
    pub(super) fn placed(&self) -> Placement {
        // Without a dart's own seam, for the reason `Sewn::dart` gives: it
        // joins a piece to itself, which is the shape the walk refuses, and a
        // garment would stop being placed on the body the moment anybody cut
        // one.
        let sewn: Vec<Sewn> = self.sewn().0.into_iter().filter(|one| !one.dart).collect();
        place::around(
            &sewn,
            &self.pipelines(),
            &self.collider,
            self.elastics().band,
            (&self.declared(), &self.pinned()),
        )
    }

    /// The document seams that could not be paired onto the cloth.
    ///
    /// A seam that refuses is left out of the solver rather than guessed at,
    /// and this is what lets the interface say so instead of drawing a garment
    /// that is quietly one seam short.
    pub fn seam_faults(&self) -> &[(SeamKey, SeamFault)] {
        &self.faults
    }

    /// The station the document declared and the ring a garment of this size
    /// would have been put on, when the release had to choose between them.
    ///
    /// Nothing at all when the two agree, when nothing was declared, and for a
    /// body that carries no rings to disagree about. The declaration won, so
    /// this is not a complaint about the garment — it is the one case where
    /// what a person asked for and what their cloth measures point at different
    /// parts of a body, and neither of those may be taken in silence.
    pub fn worn_elsewhere(&self) -> Option<&Elsewhere> {
        self.elsewhere.as_ref()
    }

    /// The ring the release had to open furthest past the cloth that goes
    /// round it, when one came off much longer than that cloth.
    ///
    /// Nothing at all for every garment the tree places today, which is the
    /// point: a garment drafted to the body wearing it comes off a hoop within
    /// a hundredth of its own cloth, and the widest reading in the suite is a
    /// blouse cut for a narrower chest at 1.161×. What lights this is a panel
    /// declared at a ring it does not belong on — the opening walk lets the
    /// surface out until it clears the person, and a garment covering half its
    /// ring is the shape that mistake takes. Quiet on agreement for the reason
    /// [`Session::worn_elsewhere`] is quiet on a garment the body agrees with.
    pub fn loose_ring(&self) -> Option<&Loose> {
        self.loose.as_ref()
    }

    /// How many pieces of the product the release could not put on the body.
    ///
    /// Zero for every garment the seams chain, which is nearly all of them, and
    /// zero for a lone panel nobody sewed and nobody hung: that one has no
    /// partner to be placed against and is not waiting to be. What is left is a
    /// product one of whose rings carries a piece with three of its own seams —
    /// a chain has room for two — and a component that declares nothing and is
    /// sewn to nothing that does, which has no ring of a person to be on. Those
    /// pieces are let go flat as they always were, and the number says so
    /// instead of nothing: a garment overhead is worth a reading, and refusing
    /// to drape it is worth less than one.
    pub fn adrift(&self) -> usize {
        self.adrift
    }

    /// Every sewn pair, as indices into the snapshot's positions.
    ///
    /// Plain indices and not the solver's own type: a client can measure a
    /// seam, or draw one, without the sim's vocabulary crossing to it.
    pub fn sewn_pairs(&self) -> Vec<(u32, u32)> {
        let mut pairs = Vec::new();
        for one in &self.sewn().0 {
            pairs.extend(one.a.iter().copied().zip(one.b.iter().copied()));
        }
        pairs
    }

    /// Where the product on the stand is let go, interleaved xyz.
    ///
    /// Recomputed from the meshes standing there now, through the very call
    /// that seeds the solver, so the two cannot drift apart. The interface
    /// never asks for it: it is how a test measures a release against the body
    /// it was released around.
    pub fn released(&self) -> Vec<f32> {
        let state = couture::drop_all(
            &self.pipelines(),
            self.collider.release_height(),
            &self.rings(),
        );
        let mut out = Vec::with_capacity(state.len() * 3);
        for i in 0..state.len() {
            out.extend([state.px[i], state.py[i], state.pz[i]]);
        }
        out
    }
}

/// Concatenates every paired seam into the one set the solver holds.
///
/// The stiffness and the cap are left where the schedule puts them: the sim
/// thread ramps them as the drape runs, and a number written here would be
/// overwritten on the next substep anyway.
fn stitches(sewn: &[Sewn]) -> Seams {
    let mut seams = Seams {
        compliance: couture::sewing_at(0).0,
        max_step: SEAM_STEP,
        iterations: SEAM_PASSES,
        ..Seams::default()
    };
    for one in sewn {
        seams.a.extend_from_slice(&one.a);
        seams.b.extend_from_slice(&one.b);
    }
    seams
}

/// The pattern abscissae a run of sewn vertices begins and ends at, in the
/// piece's own frame.
///
/// The paired ends and not the stretch's widest reach, because what this is
/// for is a correspondence: the first vertex of one side is sewn to the first
/// of the other, so these two pairs of numbers say how an abscissa of one
/// piece is read on the piece it is sewn to.
fn ends_at(pipe: &ShapePipeline, run: &[u32], base: u32) -> [f64; 2] {
    let abscissa = |v: u32| pipe.pos2d[(v - base) as usize][0];
    match (run.first(), run.last()) {
        (Some(&head), Some(&tail)) => [abscissa(head), abscissa(tail)],
        _ => [0.0, 0.0],
    }
}

/// Mean pattern abscissa of a run of sewn vertices, in the piece's own frame.
fn mean_at(pipe: &ShapePipeline, run: &[u32], base: u32) -> f64 {
    if run.is_empty() {
        return 0.0;
    }
    let sum: f64 = run
        .iter()
        .map(|&v| pipe.pos2d[(v - base) as usize][0])
        .sum();
    sum / run.len() as f64
}
