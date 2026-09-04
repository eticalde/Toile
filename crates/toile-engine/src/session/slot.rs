use crate::couture::{RestStateError, ShapePipeline};

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
