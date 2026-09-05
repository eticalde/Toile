use toile_mesh::transfer::Locator;
use toile_sim::xpbd::{DistanceConstraints, State};

use super::pipeline::{COMPLIANCE, ShapePipeline};

/// A re-meshed piece, and everything the solver needs to inherit the drape it
/// already has.
///
/// The message is self-contained on purpose: the old mesh travels inside the
/// locator, so the thread that builds a swap and the thread that applies it
/// share nothing at all, and the piece being replaced stays editable while the
/// rebuild runs.
#[derive(Debug)]
pub struct MeshSwap {
    /// The old mesh's rest space, for finding the new vertices inside it.
    pub locator: Locator,
    /// Rest position of every vertex of the new mesh.
    pub pos2d: Vec<[f64; 2]>,
    /// Triangles of the new mesh, indexing `pos2d`.
    pub tris: Vec<u32>,
    /// Distance constraints of the new mesh.
    pub cons: DistanceConstraints,
}

impl MeshSwap {
    /// The message that carries the drape of one mesh onto `new`.
    ///
    /// The old mesh arrives as its rest positions and its triangles rather
    /// than as its pipeline: the rebuild runs off the interface thread, and
    /// the pipeline it replaces is still in use there.
    pub fn new(
        old_pos2d: &[[f64; 2]],
        old_tris: &[u32],
        new: &ShapePipeline,
        compliance: f32,
    ) -> MeshSwap {
        MeshSwap {
            locator: Locator::build(old_pos2d, old_tris),
            pos2d: new.pos2d.clone(),
            tris: new.tris.clone(),
            cons: new.constraints(compliance),
        }
    }
}

/// Carries the live drape onto the re-meshed piece.
///
/// Each vertex of the new mesh is located in the old mesh's 2D rest space and
/// its position and velocity interpolated barycentrically, so a topology
/// change continues the drape instead of restarting it.
pub fn onto(swap: &MeshSwap, old_state: &State) -> State {
    let mut s = State::new(swap.pos2d.len());
    for (i, p) in swap.pos2d.iter().enumerate() {
        let (t, b) = swap.locator.locate(*p);
        let [ia, ib, ic] = swap.locator.triangle(t).map(|v| v as usize);
        let lerp = |src: &[f32], out: &mut [f32]| {
            out[i] = (b[0] * f64::from(src[ia])
                + b[1] * f64::from(src[ib])
                + b[2] * f64::from(src[ic])) as f32;
        };
        lerp(&old_state.px, &mut s.px);
        lerp(&old_state.py, &mut s.py);
        lerp(&old_state.pz, &mut s.pz);
        lerp(&old_state.vx, &mut s.vx);
        lerp(&old_state.vy, &mut s.vy);
        lerp(&old_state.vz, &mut s.vz);
    }
    s
}

/// [`onto`] for a caller holding both pipelines that wants only the carried
/// state: the command line and the benchmarks.
pub fn transfer_state(old: &ShapePipeline, old_state: &State, new: &ShapePipeline) -> State {
    onto(
        &MeshSwap::new(&old.pos2d, &old.tris, new, COMPLIANCE),
        old_state,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rectangle, corners only, and the same rectangle with a node added at
    /// the middle of its hem: a topology edit as small as one can be.
    fn rectangles() -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
        let plain = vec![[0.0, 0.0], [0.30, 0.0], [0.30, 0.20], [0.0, 0.20]];
        let mut cut = plain.clone();
        cut.insert(1, [0.15, 0.0]);
        (plain, cut)
    }

    #[test]
    fn a_swap_carries_the_state_of_the_mesh_it_replaces() {
        let (plain, cut) = rectangles();
        let old = ShapePipeline::build(&plain, 16, 0.01).expect("the rectangle is finite");
        let new = ShapePipeline::build(&cut, 17, 0.01).expect("the rectangle is finite");
        let mut state = State::new(old.pos2d.len());
        // A rigid state: every vertex carries the same numbers, so any
        // barycentric mixture of them has to give those numbers back.
        for i in 0..state.len() {
            state.py[i] = 0.35;
            state.vz[i] = -1.25;
        }
        let swap = MeshSwap::new(&old.pos2d, &old.tris, &new, COMPLIANCE);
        let carried = onto(&swap, &state);
        assert_eq!(carried.len(), new.pos2d.len());
        assert!(carried.py.iter().all(|y| (y - 0.35).abs() < 1.0e-6));
        assert!(carried.vz.iter().all(|v| (v + 1.25).abs() < 1.0e-6));
        assert_eq!(swap.cons.rest.len(), new.edges.len());
    }

    #[test]
    fn transfer_state_and_onto_agree() {
        let (plain, cut) = rectangles();
        let old = ShapePipeline::build(&plain, 16, 0.01).expect("the rectangle is finite");
        let new = ShapePipeline::build(&cut, 17, 0.01).expect("the rectangle is finite");
        let mut state = State::new(old.pos2d.len());
        for (i, p) in old.pos2d.iter().enumerate() {
            state.px[i] = p[0] as f32;
            state.pz[i] = p[1] as f32;
        }
        let wrapper = transfer_state(&old, &state, &new);
        let direct = onto(
            &MeshSwap::new(&old.pos2d, &old.tris, &new, COMPLIANCE),
            &state,
        );
        assert_eq!(wrapper.px, direct.px);
        assert_eq!(wrapper.pz, direct.pz);
    }
}
