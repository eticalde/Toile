use crate::couture::{RestStateError, ShapePipeline};
use crate::draft::PieceKey;

/// One piece's mesh, and the topology it was meshed from.
///
/// The count is derivation state, not document state: it is never saved,
/// never undone, and never part of a diff. It exists so that a shape edit
/// arriving at a mesh built from a different set of nodes is an error rather
/// than a silently corrupt warm start.
#[derive(Debug)]
pub struct PieceSlot {
    pipeline: ShapePipeline,
    topology: u64,
}

impl PieceSlot {
    /// A slot around a mesh built at `topology`.
    pub fn new(pipeline: ShapePipeline, topology: u64) -> PieceSlot {
        PieceSlot { pipeline, topology }
    }

    /// The mesh and the rest state it compiles.
    pub fn pipeline(&self) -> &ShapePipeline {
        &self.pipeline
    }

    /// The topology count the mesh was built at.
    pub fn topology(&self) -> u64 {
        self.topology
    }

    /// Puts a freshly built mesh in the slot, at the count it was built from.
    ///
    /// The count moves with the mesh and never on its own: that is what makes
    /// a mismatch mean a real disagreement rather than a missed bookkeeping
    /// step somewhere else.
    pub fn swap_in(&mut self, pipeline: ShapePipeline, topology: u64) {
        self.pipeline = pipeline;
        self.topology = topology;
    }

    /// Recompiles an edited contour into rest lengths.
    ///
    /// # Errors
    /// `RestStateError::PointCount` when the contour has gained or lost a
    /// node since the mesh was built.
    pub fn derive(&mut self, contour: &[[f64; 2]]) -> Result<&[f32], RestStateError> {
        self.pipeline.derive(contour)
    }

    /// Moves the count the mesh claims to have been built at.
    #[cfg(test)]
    pub fn set_topology(&mut self, topology: u64) {
        self.topology = topology;
    }
}

/// One piece of the product as the session drapes it.
///
/// The pieces are held in a row, and that row is the order the combined solver
/// state concatenates them in: a piece's vertices begin past every piece before
/// it. Nothing here writes that base down — it is summed from the meshes when
/// it is wanted, so it cannot drift from the state it describes.
#[derive(Debug)]
pub(super) struct Draped {
    /// The document piece this is, absent for the demo scene's own panel:
    /// that bodice belongs to no document, so nothing can name it in an edit.
    pub(super) piece: Option<PieceKey>,
    /// Its mesh, and the topology that mesh was built at.
    pub(super) slot: PieceSlot,
    /// The contour it was meshed from, in metres.
    pub(super) contour: Vec<[f64; 2]>,
    /// How many rebuilds of this piece are out with the mesher.
    ///
    /// Counted per piece and not per session: a shape edit on one piece is
    /// perfectly derivable while another piece is being re-meshed, and it is
    /// only the piece under the mesher whose contour belongs to a topology its
    /// mesh does not have yet.
    pub(super) rebuilds: u32,
    /// A shape edit arrived for this piece while its mesh was being rebuilt,
    /// and has not reached the solver yet.
    pub(super) moved_while_meshing: bool,
}

impl Draped {
    /// A piece freshly meshed: no rebuild out, and nothing deferred.
    pub(super) fn new(piece: Option<PieceKey>, slot: PieceSlot, contour: Vec<[f64; 2]>) -> Draped {
        Draped {
            piece,
            slot,
            contour,
            rebuilds: 0,
            moved_while_meshing: false,
        }
    }
}
