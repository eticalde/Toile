use thiserror::Error;
use toile_mesh::cdt::MeshError;

use crate::couture::RestStateError;
use crate::draft::{Defect, DraftError, PieceKey};

/// What stops a session from starting, or from taking an edit.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SessionError {
    /// A document with nothing on the table.
    #[error("the document has no piece to drape")]
    NoPiece,
    /// A piece that does not resolve into a contour.
    #[error("the piece cannot be drawn: {defect}")]
    Defective {
        /// The piece at fault.
        piece: PieceKey,
        /// The first thing wrong with it.
        defect: Defect,
    },
    /// An edit sent to a session that drapes the demo scene.
    #[error("this session drapes a demo scene, so it has no document to edit")]
    NoDocument,
    /// The document refused the edit.
    #[error(transparent)]
    Draft(#[from] DraftError),
    /// The mesher refused the contour.
    #[error(transparent)]
    Mesh(#[from] MeshError),
    /// The contour no longer matches the mesh built from it.
    #[error(transparent)]
    RestState(#[from] RestStateError),
    /// A shape edit that reached a mesh built from another topology. Loud on
    /// purpose: warm-starting across it would corrupt the drape in silence.
    #[error("piece {} was meshed at topology {expected}, but the draft is at {got}", piece.index())]
    TopologyMismatch {
        /// The piece whose mesh is out of date.
        piece: PieceKey,
        /// The count the mesh was built at.
        expected: u64,
        /// The count the draft is at now.
        got: u64,
    },
}
