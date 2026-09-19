use toile_sim::xpbd::Seams;

use super::seam::{SeamFault, pair_seam_anchored};
use super::{Session, place};
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
    /// The paired vertices, as indices into the combined state.
    pub(super) a: Vec<u32>,
    pub(super) b: Vec<u32>,
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
                    sides: [ia, ib],
                    at: [
                        mean_at(pipes[ia], &a, bases[ia]),
                        mean_at(pipes[ib], &b, bases[ib]),
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

    /// Where the product is let go on the body, when the seams place it.
    ///
    /// The ring itself and not only its effect: a client showing where a
    /// garment was put, and a test measuring it against the body, both need
    /// the centre, the size and the height that were chosen.
    pub fn layout(&self) -> Option<couture::Layout> {
        place::around(
            &self.sewn().0,
            &self.pipelines(),
            &self.collider,
            self.elastics().band,
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
            self.layout().as_ref(),
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
