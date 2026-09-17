mod collider;
mod edit;
mod error;
mod open;
mod pieces;
mod place;
mod remesh;
mod seam;
mod sew;
mod slot;
mod spawn;

use std::sync::Arc;

pub use error::SessionError;
use remesh::Remesher;
pub use seam::{SeamFault, pair_seam_anchored};
use slot::Draped;
pub use slot::PieceSlot;
use toile_sim::xpbd::DistanceConstraints;

use crate::body::Collider;
use crate::couture::{self, COMPLIANCE, ShapePipeline};
use crate::draft::{Draft, PieceKey, SeamKey};
use crate::sync::{SimHandle, Snapshot};

/// Simulated seconds per substep.
const DT: f32 = 1.0 / 600.0;

/// Substeps per published frame.
const SUBSTEPS_PER_TICK: u32 = 10;

/// A live editing session: a product draping, edited in place.
///
/// This is the whole surface a client gets. No solver type crosses it, which
/// is what lets the desktop app depend on the engine alone.
pub struct Session {
    /// Every piece on the stand, in the order the one combined solver state
    /// concatenates them; empty while a document has nothing draping yet.
    draping: Vec<Draped>,
    /// Their triangles as one list, each piece's rebased past the vertices of
    /// the pieces before it, so it indexes the combined state.
    tris: Vec<u32>,
    /// The document this session edits, when it was opened from one.
    draft: Option<Draft>,
    /// The sim thread, spawned only once a piece drapes.
    handle: Option<SimHandle>,
    /// The body the drape falls on. Held here and not only on the sim thread
    /// because a piece drawn later has to be let go over this body rather
    /// than over the one the demo scene ships with.
    collider: Collider,
    /// The mesher, started the first time a topology edit needs it: a session
    /// that only ever moves points never pays for a thread.
    remesher: Option<Remesher>,
    /// The document seams that would not pair onto the cloth, as of the last
    /// time the product was sewn. Kept rather than recomputed per frame: the
    /// interface asks every frame, and the answer only changes when something
    /// re-pairs them.
    faults: Vec<(SeamKey, SeamFault)>,
    generation: u64,
    /// The generation the meshes on the table were last installed at, for
    /// telling a snapshot of an old triangulation from one of these meshes.
    mesh_generation: u64,
    revision: u64,
    /// How long the last recompile took, for the status bar.
    pub last_derive_ms: f64,
    /// How long the last rebuild took, for the status bar.
    pub last_remesh_ms: f64,
}

impl Session {
    /// The document this session edits, when it was opened from one.
    pub fn draft(&self) -> Option<&Draft> {
        self.draft.as_ref()
    }

    /// The first piece of the product that drapes, once one does.
    ///
    /// Every piece the document holds drapes now, so this is the one a panel
    /// falls back to rather than the only one on the stand;
    /// [`Session::pieces`] is the whole list.
    pub fn piece(&self) -> Option<PieceKey> {
        self.draping.first().and_then(|held| held.piece)
    }

    /// Every document piece on the stand, in the order the combined solver
    /// state concatenates them.
    ///
    /// Empty for the demo scene, whose panel belongs to no document.
    pub fn pieces(&self) -> Vec<PieceKey> {
        self.draping.iter().filter_map(|held| held.piece).collect()
    }

    /// Where a piece's vertices begin in the combined solver state.
    ///
    /// A seam joins two vertices of that one state, so each side of one has to
    /// be offset by its own piece's base — which is exactly what
    /// [`pair_seam_anchored`] asks for. `None` for a piece that does not drape.
    pub fn offset(&self, piece: PieceKey) -> Option<u32> {
        let at = self.index_of(piece)?;
        couture::offsets(&self.pipelines()).get(at).copied()
    }

    /// How many times the document on this table has changed.
    ///
    /// It counts every edit, undo, redo and refusal, and nothing else. A
    /// client that wrote the document to a file remembers the number it wrote
    /// at, and that is the whole of what an unsaved change is.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// One piece's control contour, in metres of pattern space; empty for a
    /// piece that does not drape.
    ///
    /// Suffixed for the same reason [`crate::draft::Draft::outline_m`] is: a
    /// client holds this and the draft's centimetres as one type, and only the
    /// name says which of the two it is holding.
    pub fn contour_m(&self, piece: PieceKey) -> &[[f64; 2]] {
        match self.index_of(piece) {
            Some(at) => self.draping[at].contour.as_slice(),
            None => &[],
        }
    }

    /// Every draping piece's control contour, in the order the combined state
    /// holds them. The demo scene's own panel is in here too, unnamed.
    pub fn contours_m(&self) -> impl Iterator<Item = &[[f64; 2]]> {
        self.draping.iter().map(|held| held.contour.as_slice())
    }

    /// The whole product's triangles, indexing the snapshot's positions; empty
    /// while nothing drapes.
    pub fn triangles(&self) -> &[u32] {
        &self.tris
    }

    /// The whole product's vertex count, for sizing render buffers; zero while
    /// nothing drapes.
    pub fn n_vertices(&self) -> usize {
        self.draping
            .iter()
            .map(|held| held.slot.pipeline().pos2d.len())
            .sum()
    }

    /// The generation the meshes on the table were last installed at.
    ///
    /// A snapshot carrying an earlier generation was published before the last
    /// mesh swapped in: its positions belong to the old triangulation, even
    /// when the vertex counts happen to agree.
    pub fn mesh_generation(&self) -> u64 {
        self.mesh_generation
    }

    /// The latest snapshot from the sim thread; an empty one while nothing
    /// drapes or before the first tick.
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.handle
            .as_ref()
            .map_or_else(|| Arc::new(Snapshot::default()), SimHandle::snapshot)
    }

    /// Whether there is a sim thread at all.
    ///
    /// Nothing drapes until a piece meshes, so a blank table has no thread and
    /// [`Session::snapshot`] answers with the empty snapshot — whose
    /// `converged` is false, the same value a simulation still working reports.
    /// Anything that speaks about the sim asks this first, or it speaks about a
    /// thread that does not exist.
    pub fn simulating(&self) -> bool {
        self.handle.is_some()
    }

    /// True when the sim has slept on the latest edit: nothing left to
    /// animate.
    ///
    /// The published frame's own verdict is not enough: it may have been
    /// captured before the last edit reached the sim thread.
    pub fn settled(&self) -> bool {
        let Some(handle) = self.handle.as_ref() else {
            return true;
        };
        let snap = handle.snapshot();
        snap.converged && snap.generation == self.generation
    }

    /// Where a piece sits among the ones on the stand.
    fn index_of(&self, piece: PieceKey) -> Option<usize> {
        self.draping
            .iter()
            .position(|held| held.piece == Some(piece))
    }

    /// Every piece's mesh, in the order the combined state holds them.
    fn pipelines(&self) -> Vec<&ShapePipeline> {
        self.draping
            .iter()
            .map(|held| held.slot.pipeline())
            .collect()
    }

    /// The whole product's stretch constraints, as the solver holds them.
    fn constraints(&self) -> DistanceConstraints {
        couture::combine_constraints(&self.pipelines(), COMPLIANCE)
    }

    /// Rebuilds the triangle list the viewer draws, after a mesh changed.
    fn relist(&mut self) {
        let tris = couture::combine_triangles(&self.pipelines());
        self.tris = tris;
    }
}

#[cfg(test)]
mod tests;
