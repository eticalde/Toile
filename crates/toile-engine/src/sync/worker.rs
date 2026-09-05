use std::sync::Arc;

use thiserror::Error;
use toile_sim::xpbd::{self, DistanceConstraints, KineticDamper, SdfGrid, Seams, State};

use crate::couture::{MeshSwap, onto};

/// Sleep threshold on mean kinetic energy per vertex, about 2 mm/s RMS: one
/// loose vertex fluttering must not keep the whole garment awake.
const SLEEP_ENERGY_PER_VERT: f32 = 2.0e-6;

/// Consecutive quiet ticks required before sleeping. More than one, because a
/// kinetic-damping zero leaves velocities at zero without the cloth actually
/// being at equilibrium.
const QUIET_TICKS_TO_SLEEP: u32 = 3;

/// Why the sim thread would not take a message.
///
/// A refusal is loud rather than silent for one reason: every case here is a
/// message compiled against a mesh the solver has already replaced, and taking
/// it would warm-start the drape over a topology that no longer exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum StaleMessage {
    /// A message from before the one the solver is already running.
    #[error("a message at generation {got} arrived after generation {applied}")]
    Generation {
        /// The generation the solver is at.
        applied: u64,
        /// The generation the message names.
        got: u64,
    },
    /// Rest lengths for another mesh's constraints.
    #[error("{got} rest lengths for a mesh of {expected} constraints")]
    RestCount {
        /// Constraints the solver holds.
        expected: usize,
        /// Rest lengths the message carries.
        got: usize,
    },
}

/// What the sim thread publishes after every tick.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    /// Last rest-state generation applied before these substeps.
    pub generation: u64,
    /// Substeps run since the thread started.
    pub substeps: u64,
    /// The sim is asleep, waiting for an edit.
    pub converged: bool,
    /// Interleaved xyz positions.
    pub positions: Vec<f32>,
    /// Interleaved xyz vertex normals.
    pub normals: Vec<f32>,
    /// The last message the sim refused, if it ever refused one.
    pub refused: Option<StaleMessage>,
}

/// The simulation, owned exclusively by its thread.
pub(super) struct Sim {
    state: State,
    cons: DistanceConstraints,
    seams: Seams,
    sdf: SdfGrid,
    tris: Vec<u32>,
    dt: f32,
    substeps_per_tick: u32,
    damper: KineticDamper,
    generation: u64,
    substeps: u64,
    quiet_ticks: u32,
    converged: bool,
    refused: Option<StaleMessage>,
}

impl Sim {
    pub(super) fn new(
        state: State,
        cons: DistanceConstraints,
        sdf: SdfGrid,
        tris: Vec<u32>,
        dt: f32,
        substeps_per_tick: u32,
    ) -> Self {
        Self {
            state,
            cons,
            seams: Seams::default(),
            sdf,
            tris,
            dt,
            substeps_per_tick,
            damper: KineticDamper::new(),
            generation: 0,
            substeps: 0,
            quiet_ticks: 0,
            converged: false,
            refused: None,
        }
    }

    pub(super) fn converged(&self) -> bool {
        self.converged
    }

    /// Hot-swaps the rest state and wakes the sim.
    ///
    /// # Errors
    /// `StaleMessage` when the rest lengths were compiled against a mesh the
    /// solver has already left behind.
    pub(super) fn apply_rests(
        &mut self,
        generation: u64,
        rests: &[f32],
    ) -> Result<(), StaleMessage> {
        self.fresh(generation)?;
        if rests.len() != self.cons.rest.len() {
            return Err(StaleMessage::RestCount {
                expected: self.cons.rest.len(),
                got: rests.len(),
            });
        }
        self.cons.rest.copy_from_slice(rests);
        self.wake(generation);
        Ok(())
    }

    /// Puts the piece on a new mesh, carrying the drape onto it.
    ///
    /// The mailbox is drained between ticks, never inside one, so the state
    /// this replaces is always a whole substep's worth: the transfer never
    /// reads positions halfway through their integration.
    ///
    /// # Errors
    /// `StaleMessage::Generation` when a later message has already been
    /// applied, which means this rebuild was superseded before it landed.
    pub(super) fn apply_swap(
        &mut self,
        generation: u64,
        swap: Box<MeshSwap>,
    ) -> Result<(), StaleMessage> {
        self.fresh(generation)?;
        self.state = onto(&swap, &self.state);
        let MeshSwap { tris, cons, .. } = *swap;
        self.cons = cons;
        self.tris = tris;
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
        self.converged = false;
        self.quiet_ticks = 0;
        self.damper.reset();
    }

    /// Advances one tick and updates the convergence verdict.
    pub(super) fn tick(&mut self) {
        let inv_n = 1.0 / self.state.len() as f32;
        let mut e_avg = 0.0f32;
        for _ in 0..self.substeps_per_tick {
            xpbd::substep(&mut self.state, &self.cons, &self.seams, &self.sdf, self.dt);
            self.substeps += 1;
            e_avg = self.damper.observe(&mut self.state) * inv_n;
        }
        if e_avg < SLEEP_ENERGY_PER_VERT {
            self.quiet_ticks += 1;
        } else {
            self.quiet_ticks = 0;
        }
        self.converged = self.quiet_ticks >= QUIET_TICKS_TO_SLEEP;
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
            converged: self.converged,
            positions,
            normals,
            refused: self.refused,
        })
    }
}

#[cfg(test)]
mod tests;
