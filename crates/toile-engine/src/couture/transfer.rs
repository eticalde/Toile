use toile_mesh::transfer::Locator;
use toile_sim::xpbd::{DistanceConstraints, Seams, State};

use super::pipeline::{COMPLIANCE, ShapePipeline};

/// A re-meshed piece, and everything the solver needs to inherit the drape it
/// already has.
///
/// The message is self-contained on purpose: the old mesh travels inside the
/// locator, so the thread that builds a swap and the thread that applies it
/// share nothing at all, and the piece being replaced stays editable while the
/// rebuild runs.
///
/// A product is one state holding every piece, so the swap also names the run
/// of that state the rebuilt piece occupies. The constraints and the triangles
/// are the whole product's, because a piece that gained or lost a vertex moves
/// the base of every piece after it.
#[derive(Debug)]
pub struct MeshSwap {
    /// The old mesh's rest space, for finding the new vertices inside it.
    pub locator: Locator,
    /// Rest position of every vertex of the new mesh.
    pub pos2d: Vec<[f64; 2]>,
    /// Triangles of the whole product, indexing the combined state.
    pub tris: Vec<u32>,
    /// Distance constraints of the whole product, likewise.
    pub cons: DistanceConstraints,
    /// What is sewn to what, likewise.
    ///
    /// Every seam of the product travels with the swap, not only the rebuilt
    /// piece's: a rebuild hands that piece a whole new set of vertices, and
    /// moves the base of every piece standing after it, so there is no seam
    /// whose indices can be assumed to have survived.
    pub seams: Seams,
    /// Where the re-meshed piece begins in the state this replaces.
    pub at: u32,
    /// How many vertices stood there before.
    pub replacing: u32,
}

impl MeshSwap {
    /// The message that carries the drape of one mesh onto `new`, for a piece
    /// that is the whole of the solver's state.
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
            seams: Seams::default(),
            at: 0,
            replacing: old_pos2d.len() as u32,
        }
    }
}

/// Carries the live drape onto the re-meshed piece, leaving every other piece
/// of the product where it was.
///
/// Each vertex of the new mesh is located in the old mesh's 2D rest space and
/// its position and velocity interpolated barycentrically, so a topology
/// change continues the drape instead of restarting it. The pieces before and
/// after it are copied across untouched, at whatever indices the new vertex
/// count leaves them.
pub fn onto(swap: &MeshSwap, old_state: &State) -> State {
    let (at, taken) = (swap.at as usize, swap.replacing as usize);
    let past = at + taken;
    let mut s = State::new(old_state.len() - taken + swap.pos2d.len());
    carry(old_state, 0, at, &mut s, 0);
    for (i, p) in swap.pos2d.iter().enumerate() {
        let (t, b) = swap.locator.locate(*p);
        let [ia, ib, ic] = swap.locator.triangle(t).map(|v| at + v as usize);
        let lerp = |src: &[f32], out: &mut [f32]| {
            out[at + i] = (b[0] * f64::from(src[ia])
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
    let tail = old_state.len() - past;
    carry(old_state, past, tail, &mut s, at + swap.pos2d.len());
    s
}

/// Copies a run of the state across untouched.
///
/// A piece the rebuild did not name keeps the position and the motion it had,
/// bit for bit. Only the index it sits at moves, and only when a piece before
/// it changed how many vertices it has.
fn carry(from: &State, at: usize, count: usize, into: &mut State, to: usize) {
    for (src, dst) in [
        (&from.px, &mut into.px),
        (&from.py, &mut into.py),
        (&from.pz, &mut into.pz),
        (&from.vx, &mut into.vx),
        (&from.vy, &mut into.vy),
        (&from.vz, &mut into.vz),
    ] {
        dst[to..to + count].copy_from_slice(&src[at..at + count]);
    }
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

    /// The two meshes a swap runs between.
    fn meshes() -> (ShapePipeline, ShapePipeline) {
        let (plain, cut) = rectangles();
        let finite = "the rectangle is finite";
        (
            ShapePipeline::build(&plain, 16, 0.01).expect(finite),
            ShapePipeline::build(&cut, 17, 0.01).expect(finite),
        )
    }

    #[test]
    fn a_swap_carries_the_state_of_the_mesh_it_replaces() {
        let (old, new) = meshes();
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
        assert_eq!((swap.at, swap.replacing), (0, old.pos2d.len() as u32));
    }

    /// A rebuild of one piece of a product touches only the run it names: the
    /// vertices before and after it come back bit for bit, moved by however
    /// many the rebuilt piece gained and by nothing else.
    #[test]
    fn a_swap_inside_a_product_leaves_the_other_pieces_alone() {
        let (old, new) = meshes();
        let (head, tail, n) = (7usize, 5usize, old.pos2d.len());
        let mut state = State::new(head + n + tail);
        for i in 0..state.len() {
            state.px[i] = i as f32;
            state.vy[i] = -(i as f32);
        }
        let swap = MeshSwap {
            at: head as u32,
            replacing: n as u32,
            ..MeshSwap::new(&old.pos2d, &old.tris, &new, COMPLIANCE)
        };

        let carried = onto(&swap, &state);
        let moved = head + new.pos2d.len();
        assert_eq!(carried.len(), moved + tail);
        assert_eq!(carried.px[..head], state.px[..head]);
        assert_eq!(carried.vy[..head], state.vy[..head]);
        assert_eq!(carried.px[moved..], state.px[head + n..]);
        assert_eq!(carried.vy[moved..], state.vy[head + n..]);
    }

    #[test]
    fn transfer_state_and_onto_agree() {
        let (old, new) = meshes();
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
