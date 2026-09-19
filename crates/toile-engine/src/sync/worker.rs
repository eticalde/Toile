use std::sync::Arc;

use toile_sim::xpbd::{self, DistanceConstraints, KineticDamper, Seams, Stage, State};

use super::handle::Scene;
use super::report::{Snapshot, StaleMessage};
use super::sleep::Sleep;
use crate::couture::{self, MeshSwap, onto};

/// The simulation, owned exclusively by its thread.
pub(super) struct Sim {
    state: State,
    cons: DistanceConstraints,
    seams: Seams,
    scene: Scene,
    tris: Vec<u32>,
    dt: f32,
    substeps_per_tick: u32,
    damper: KineticDamper,
    generation: u64,
    substeps: u64,
    sleep: Sleep,
    refused: Option<StaleMessage>,
}

impl Sim {
    pub(super) fn new(
        state: State,
        cons: DistanceConstraints,
        seams: Seams,
        scene: Scene,
        tris: Vec<u32>,
        dt: f32,
        substeps_per_tick: u32,
    ) -> Self {
        Self {
            state,
            cons,
            seams,
            scene,
            tris,
            dt,
            substeps_per_tick,
            damper: KineticDamper::new(),
            generation: 0,
            substeps: 0,
            sleep: Sleep::default(),
            refused: None,
        }
    }

    pub(super) fn converged(&self) -> bool {
        self.sleep.asleep()
    }

    /// Hot-swaps the rest state and what is sewn to what, and wakes the sim.
    ///
    /// The three arrive together because a shape edit moves all of them: the
    /// rest lengths because the cloth changed shape, the compliances because
    /// an elastic is a stretch of that same contour pulling harder than the
    /// cloth around it, and the seams because an anchor is one of its nodes.
    ///
    /// # Errors
    /// `StaleMessage` when the message was compiled against a mesh the solver
    /// has already left behind — by generation, by rest count, or by naming a
    /// vertex it does not hold.
    pub(super) fn apply_rests(
        &mut self,
        generation: u64,
        rests: &[f32],
        compliance: &[f32],
        seams: Seams,
    ) -> Result<(), StaleMessage> {
        self.fresh(generation)?;
        if rests.len() != self.cons.rest.len() || compliance.len() != self.cons.compliance.len() {
            return Err(StaleMessage::RestCount {
                expected: self.cons.rest.len(),
                got: rests.len(),
            });
        }
        holds(&seams, self.state.len())?;
        self.cons.rest.copy_from_slice(rests);
        self.cons.compliance.copy_from_slice(compliance);
        self.seams = seams;
        self.wake(generation);
        Ok(())
    }

    /// Puts one piece of the product on a new mesh, carrying the drape onto
    /// it and leaving the other pieces where they are.
    ///
    /// The mailbox is drained between ticks, never inside one, so the state
    /// this replaces is always a whole substep's worth: the transfer never
    /// reads positions halfway through their integration.
    ///
    /// # Errors
    /// `StaleMessage::Generation` when a later message has already been
    /// applied, which means this rebuild was superseded before it landed, and
    /// `SwapRange` or `SeamRange` when it names a vertex past the end of the
    /// state — refused rather than sliced, because a panic here would stop the
    /// drape with nothing said.
    pub(super) fn apply_swap(
        &mut self,
        generation: u64,
        swap: Box<MeshSwap>,
    ) -> Result<(), StaleMessage> {
        self.fresh(generation)?;
        let len = self.state.len();
        if swap.at as usize + swap.replacing as usize > len {
            return Err(StaleMessage::SwapRange {
                at: swap.at,
                replacing: swap.replacing,
                len,
            });
        }
        // Against the state the swap leaves behind, not the one it replaces:
        // a rebuilt piece changes how many vertices stand before every piece
        // after it, so the seams travelling with it are written in the new
        // numbering and only that one can judge them.
        holds(
            &swap.seams,
            len - swap.replacing as usize + swap.pos2d.len(),
        )?;
        self.state = onto(&swap, &self.state);
        let MeshSwap {
            tris, cons, seams, ..
        } = *swap;
        self.cons = cons;
        self.tris = tris;
        self.seams = seams;
        self.wake(generation);
        Ok(())
    }

    /// Puts the drape on another body, carrying it out of one that has
    /// swallowed it.
    ///
    /// A body that grew leaves cloth under its skin. Left for the ordinary
    /// contact solve, every such particle would be pushed the whole way out
    /// inside one substep and the velocity derived from that jump would fling
    /// the garment off. `lift_out_of` moves it onto the new skin without
    /// giving it that speed, so a measurement moved mid-drape re-drapes the
    /// garment rather than throwing it.
    ///
    /// Nothing is re-dropped: the position and the motion the cloth already
    /// had are the whole of what the person is looking at.
    ///
    /// # Errors
    /// `StaleMessage::Generation` when a later message has already been
    /// applied, which means this body was superseded before it landed.
    pub(super) fn apply_collider(
        &mut self,
        generation: u64,
        scene: Scene,
    ) -> Result<(), StaleMessage> {
        self.fresh(generation)?;
        // The ground arrives with the field: a body that got shorter stands on
        // a higher plane, and a drape left resting on the old one would hang
        // in the air beside the new body.
        self.scene = scene;
        xpbd::lift_out_of(&self.scene.sdf, &mut self.state);
        self.wake(generation);
        Ok(())
    }

    /// Records a refusal, so a client polling the snapshot can see it.
    pub(super) fn refuse(&mut self, why: StaleMessage) {
        self.refused = Some(why);
    }

    /// Whether a message names a generation the sim has not passed.
    fn fresh(&self, generation: u64) -> Result<(), StaleMessage> {
        if generation <= self.generation {
            return Err(StaleMessage::Generation {
                applied: self.generation,
                got: generation,
            });
        }
        Ok(())
    }

    /// Takes a message in and puts the cloth back in motion.
    fn wake(&mut self, generation: u64) {
        self.generation = generation;
        self.sleep.wake();
        self.damper.reset();
    }

    /// Advances one tick and asks whether the drape may sleep after it.
    ///
    /// The sewing tightens as the drape runs, and it is counted from the
    /// thread's own substeps rather than from the last edit: a product is let
    /// go with its seams open, and that is the one moment they need to be
    /// soft. A later edit moves the cloth, not the pieces apart.
    ///
    /// Gravity waits for the same moment. While the pieces are still being
    /// pulled together the garment hangs weightless, and it is let fall the
    /// substep its seams are shut — see [`couture::closing`]. A product with
    /// nothing sewn never enters the phase and falls from the first substep,
    /// exactly as it always has.
    pub(super) fn tick(&mut self) {
        self.sleep.mark(&self.state);
        for _ in 0..self.substeps_per_tick {
            let mut stage = Stage::around(&self.scene.sdf).on(self.scene.floor);
            if !self.seams.is_empty() {
                (self.seams.compliance, self.seams.max_step) = couture::sewing_at(self.substeps);
                let gap = xpbd::seam_gap(&self.state, &self.seams);
                if couture::closing(self.substeps, gap) {
                    stage = stage.weightless();
                }
            }
            // No layers: the drape a person watches does not collide with
            // itself yet. The pass exists and is proven, and what it costs per
            // substep is the whole reason it is not switched on here.
            xpbd::substep(
                &mut self.state,
                &self.cons,
                &self.seams,
                &stage,
                None,
                self.dt,
            );
            self.substeps += 1;
            self.damper.observe(&mut self.state);
        }
        self.sleep.judge(&self.state);
    }

    pub(super) fn publish(&self) -> Arc<Snapshot> {
        let n = self.state.len();
        let mut positions = Vec::with_capacity(n * 3);
        for i in 0..n {
            positions.push(self.state.px[i]);
            positions.push(self.state.py[i]);
            positions.push(self.state.pz[i]);
        }
        let mut normals = vec![0.0f32; n * 3];
        xpbd::vertex_normals(&self.state, &self.tris, &mut normals);
        Arc::new(Snapshot {
            generation: self.generation,
            substeps: self.substeps,
            converged: self.sleep.asleep(),
            positions,
            normals,
            refused: self.refused,
        })
    }
}

/// Whether every sewn vertex is one a state of `len` particles holds.
///
/// The solver indexes these directly, so one past the end is a panic on the
/// sim thread rather than a wrong drape — the same reason a swap's run is
/// checked before it is taken.
fn holds(seams: &Seams, len: usize) -> Result<(), StaleMessage> {
    match seams
        .a
        .iter()
        .chain(&seams.b)
        .copied()
        .find(|&v| v as usize >= len)
    {
        Some(vertex) => Err(StaleMessage::SeamRange { vertex, len }),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests;
