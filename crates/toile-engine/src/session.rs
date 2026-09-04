use std::sync::Arc;
use std::time::Instant;

use crate::couture::ShapePipeline;
use crate::demo;
use crate::sync::{self, SimHandle, Snapshot};

/// Simulated seconds per substep.
const DT: f32 = 1.0 / 600.0;

/// Substeps per published frame.
const SUBSTEPS_PER_TICK: u32 = 10;

/// A live editing session: one piece, draping, edited in place.
///
/// This is the whole surface a client gets. No solver type crosses it, which
/// is what lets the desktop app depend on the engine alone.
pub struct Session {
    pipeline: ShapePipeline,
    contour: Vec<[f64; 2]>,
    handle: SimHandle,
    generation: u64,
    /// How long the last recompile took, for the status bar.
    pub last_derive_ms: f64,
}

impl Session {
    /// The demo bodice, draping over the avatar on its own thread.
    pub fn demo_bodice() -> Session {
        let contour = demo::bodice_contour();
        let pipeline = demo::pipeline(&contour);
        let state = demo::drop_state(&pipeline);
        let cons = pipeline.constraints(1.0e-8);
        let handle = sync::spawn(
            state,
            cons,
            demo::avatar_sdf(),
            pipeline.tris.clone(),
            DT,
            SUBSTEPS_PER_TICK,
        );
        Session {
            pipeline,
            contour,
            handle,
            generation: 0,
            last_derive_ms: 0.0,
        }
    }

    /// The piece's control contour, in metres of pattern space.
    pub fn contour(&self) -> &[[f64; 2]] {
        &self.contour
    }

    /// Mesh triangles, indexing the snapshot's positions.
    pub fn triangles(&self) -> &[u32] {
        &self.pipeline.tris
    }

    /// Mesh vertex count, for sizing render buffers.
    pub fn n_vertices(&self) -> usize {
        self.pipeline.pos2d.len()
    }

    /// Radius of the sphere standing in for the avatar.
    pub fn avatar_radius(&self) -> f32 {
        demo::AVATAR_RADIUS
    }

    /// The latest snapshot from the sim thread; empty until the first tick.
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.handle.snapshot()
    }

    /// True when the sim has slept on the latest edit: nothing left to
    /// animate.
    ///
    /// The published frame's own verdict is not enough: it may have been
    /// captured before the last edit reached the sim thread.
    pub fn settled(&self) -> bool {
        let snap = self.handle.snapshot();
        snap.converged && snap.generation == self.generation
    }

    /// Moves a control point and hot-swaps the resulting rest state.
    ///
    /// Out-of-range indices are ignored. Nothing is reset: the drape carries
    /// on from where it was.
    pub fn move_point(&mut self, index: usize, to: [f64; 2]) {
        if index >= self.contour.len() {
            return;
        }
        self.contour[index] = to;
        let t = Instant::now();
        let rests = self.pipeline.derive(&self.contour).to_vec();
        self.last_derive_ms = t.elapsed().as_secs_f64() * 1000.0;
        self.generation += 1;
        self.handle.send_rests(self.generation, rests);
    }
}
