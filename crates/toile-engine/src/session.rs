mod edit;
mod error;
mod pieces;
mod remesh;
mod seam;
mod slot;
mod spawn;

use std::sync::Arc;

pub use error::SessionError;
use remesh::Remesher;
pub use seam::{SeamFault, pair_seam_anchored};
pub use slot::PieceSlot;
use spawn::{drape_piece, spawn_sim};

use crate::couture::COMPLIANCE;
use crate::demo;
use crate::draft::{Doc, Draft, MeasureSet, PieceKey};
use crate::sync::{SimHandle, Snapshot};

/// Simulated seconds per substep.
const DT: f32 = 1.0 / 600.0;

/// Substeps per published frame.
const SUBSTEPS_PER_TICK: u32 = 10;

/// The document a session edits, and the piece of it that drapes, once one
/// does: a document opened blank has pieces to draw before any of them drapes.
struct Drafted {
    draft: Draft,
    piece: Option<PieceKey>,
}

/// A live editing session: one piece, draping, edited in place.
///
/// This is the whole surface a client gets. No solver type crosses it, which
/// is what lets the desktop app depend on the engine alone.
pub struct Session {
    /// The meshed piece on the stand, absent while a document has no piece
    /// draping yet.
    slot: Option<PieceSlot>,
    contour: Vec<[f64; 2]>,
    drafted: Option<Drafted>,
    /// The sim thread, spawned only once a piece drapes.
    handle: Option<SimHandle>,
    /// The mesher, started the first time a topology edit needs it: a session
    /// that only ever moves points never pays for a thread.
    remesher: Option<Remesher>,
    /// A shape edit arrived while the mesher was working, and has not reached
    /// the solver yet.
    moved_while_meshing: bool,
    generation: u64,
    /// The generation the mesh on the table was installed at, for telling a
    /// snapshot of the old triangulation from one of this mesh.
    mesh_generation: u64,
    revision: u64,
    /// How long the last recompile took, for the status bar.
    pub last_derive_ms: f64,
    /// How long the last rebuild took, for the status bar.
    pub last_remesh_ms: f64,
}

impl Session {
    /// The demo bodice, draping over the avatar on its own thread.
    pub fn demo_bodice() -> Session {
        let contour = demo::bodice_contour();
        let pipeline = demo::pipeline(&contour);
        let state = demo::drop_state(&pipeline);
        let slot = PieceSlot::new(pipeline, 0);
        let handle = spawn_sim(&slot, state);
        Session::build(Some(slot), contour, None, Some(handle))
    }

    /// A blank document: a table with nothing drawn, ready for the first piece.
    ///
    /// The document still carries a mannequin, since every coordinate resolves
    /// against one; it simply has no pieces yet. Nothing drapes until one is
    /// drawn, so there is no mesh and no sim thread until then.
    ///
    /// # Panics
    /// Never in practice: a document with no pieces has nothing to resolve, so
    /// the draft cannot fail to build.
    pub fn blank() -> Session {
        let doc = Doc::new(MeasureSet::default());
        let draft = Draft::from_doc(doc).expect("an empty document resolves");
        let drafted = Drafted { draft, piece: None };
        Session::build(None, Vec::new(), Some(drafted), None)
    }

    /// A document draping its first piece, on its own thread.
    ///
    /// # Errors
    /// `SessionError` when the document does not resolve, draws nothing, or
    /// carries a contour the mesher refuses.
    pub fn from_doc(doc: Doc) -> Result<Session, SessionError> {
        let draft = Draft::from_doc(doc)?;
        let piece = draft
            .doc()
            .piece_keys()
            .first()
            .copied()
            .ok_or(SessionError::NoPiece)?;
        let (slot, contour, state) = drape_piece(&draft, piece)?;
        let handle = spawn_sim(&slot, state);
        let drafted = Drafted {
            draft,
            piece: Some(piece),
        };
        Ok(Session::build(
            Some(slot),
            contour,
            Some(drafted),
            Some(handle),
        ))
    }

    /// The document this session edits, when it was opened from one.
    pub fn draft(&self) -> Option<&Draft> {
        self.drafted.as_ref().map(|held| &held.draft)
    }

    /// The piece this session drapes, once one does.
    pub fn piece(&self) -> Option<PieceKey> {
        self.drafted.as_ref().and_then(|held| held.piece)
    }

    /// How many times the document on this table has changed.
    ///
    /// It counts every edit, undo, redo and refusal, and nothing else. A
    /// client that wrote the document to a file remembers the number it wrote
    /// at, and that is the whole of what an unsaved change is.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The piece's control contour, in metres of pattern space.
    pub fn contour(&self) -> &[[f64; 2]] {
        &self.contour
    }

    /// Mesh triangles, indexing the snapshot's positions; empty while nothing
    /// drapes.
    pub fn triangles(&self) -> &[u32] {
        self.slot.as_ref().map_or(&[], |slot| &slot.pipeline().tris)
    }

    /// Mesh vertex count, for sizing render buffers; zero while nothing drapes.
    pub fn n_vertices(&self) -> usize {
        self.slot
            .as_ref()
            .map_or(0, |slot| slot.pipeline().pos2d.len())
    }

    /// The generation the mesh on the table was installed at.
    ///
    /// A snapshot carrying an earlier generation was published before this
    /// mesh swapped in: its positions belong to the old triangulation, even
    /// when the vertex counts happen to agree.
    pub fn mesh_generation(&self) -> u64 {
        self.mesh_generation
    }

    /// Radius of the sphere standing in for the avatar.
    pub fn avatar_radius(&self) -> f32 {
        demo::AVATAR_RADIUS
    }

    /// The latest snapshot from the sim thread; an empty one while nothing
    /// drapes or before the first tick.
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.handle
            .as_ref()
            .map_or_else(|| Arc::new(Snapshot::default()), SimHandle::snapshot)
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

    /// Fills the session's fields; the caller decides whether a piece drapes.
    fn build(
        slot: Option<PieceSlot>,
        contour: Vec<[f64; 2]>,
        drafted: Option<Drafted>,
        handle: Option<SimHandle>,
    ) -> Session {
        Session {
            slot,
            contour,
            drafted,
            handle,
            remesher: None,
            moved_while_meshing: false,
            generation: 0,
            mesh_generation: 0,
            revision: 0,
            last_derive_ms: 0.0,
            last_remesh_ms: 0.0,
        }
    }

    /// Adopts a piece drawn into a blank document as the one that drapes,
    /// meshing it and starting the sim thread around it.
    ///
    /// # Errors
    /// `SessionError` when the piece carries a defect or a contour the mesher
    /// refuses; the table then stays blank and the drawing can be corrected.
    pub(super) fn seed_piece(&mut self, piece: PieceKey) -> Result<(), SessionError> {
        let Some(drafted) = self.drafted.as_mut() else {
            return Ok(());
        };
        let (slot, contour, state) = drape_piece(&drafted.draft, piece)?;
        drafted.piece = Some(piece);
        self.handle = Some(spawn_sim(&slot, state));
        self.slot = Some(slot);
        self.contour = contour;
        self.generation = 0;
        self.mesh_generation = 0;
        Ok(())
    }

    /// Tears the drape down to a blank table, keeping the document.
    ///
    /// The sim thread stops when its handle drops. Used when the piece that was
    /// draping leaves the document, as an undo of the first piece does.
    pub(super) fn unseed(&mut self) {
        self.handle = None;
        self.slot = None;
        self.contour = Vec::new();
        self.remesher = None;
        self.moved_while_meshing = false;
        if let Some(drafted) = self.drafted.as_mut() {
            drafted.piece = None;
        }
    }
}

#[cfg(test)]
mod tests;
