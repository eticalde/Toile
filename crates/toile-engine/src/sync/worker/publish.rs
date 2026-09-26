use std::sync::Arc;

use toile_sim::xpbd;

use super::Sim;
use crate::sync::report::{Hanging, Snapshot};

impl Sim {
    /// The frame the thread hands whoever is drawing: where the cloth is, which
    /// way it faces, and what is holding it there.
    pub(in crate::sync) fn publish(&self) -> Arc<Snapshot> {
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
            asleep: self.sleep.asleep(),
            positions,
            normals,
            refused: self.refused,
            hanging: self.hanging(),
        })
    }

    /// What the body is holding up, when it is holding anything.
    ///
    /// The scene says how many runs it holds and the anchor pass says how far
    /// off it left them, so a garment hung from nothing answers with nothing at
    /// all: the reach left behind by a pass over an empty list is a zero that
    /// measured nothing, and publishing it would have the fitting room
    /// reporting that a skirt nobody hung is sitting exactly at its rings.
    fn hanging(&self) -> Option<Hanging> {
        let gap = self.reach.filter(|_| !self.scene.hung.is_empty())?;
        Some(Hanging {
            runs: self.scene.hung.len(),
            gap,
        })
    }
}
