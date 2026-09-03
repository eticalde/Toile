use std::sync::Arc;

use toile_sim::xpbd::{self, DistanceConstraints, KineticDamper, SdfGrid, Seams, State};

/// Sleep threshold on mean kinetic energy per vertex, about 2 mm/s RMS: one
/// loose vertex fluttering must not keep the whole garment awake.
const SLEEP_ENERGY_PER_VERT: f32 = 2.0e-6;

/// Consecutive quiet ticks required before sleeping. More than one, because a
/// kinetic-damping zero leaves velocities at zero without the cloth actually
/// being at equilibrium.
const QUIET_TICKS_TO_SLEEP: u32 = 3;

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
        }
    }

    pub(super) fn converged(&self) -> bool {
        self.converged
    }

    /// Hot-swaps the rest state and wakes the sim.
    ///
    /// # Panics
    /// If `rests` does not have one entry per constraint.
    pub(super) fn apply_rests(&mut self, generation: u64, rests: &[f32]) {
        self.cons.rest.copy_from_slice(rests);
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
        })
    }
}
