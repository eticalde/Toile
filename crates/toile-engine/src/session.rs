mod edit;
mod error;
mod slot;

use std::sync::Arc;

pub use error::SessionError;
pub use slot::PieceSlot;

use crate::couture::{self, ShapePipeline};
use crate::demo;
use crate::draft::{Doc, Draft, PieceKey};
use crate::sync::{self, SimHandle, Snapshot};

/// Simulated seconds per substep.
const DT: f32 = 1.0 / 600.0;

/// Substeps per published frame.
const SUBSTEPS_PER_TICK: u32 = 10;

/// Uniform stretch compliance, until fabric presets bring anisotropy.
const COMPLIANCE: f32 = 1.0e-8;

/// The document a session edits, and the piece of it that drapes.
struct Drafted {
    draft: Draft,
    piece: PieceKey,
}

/// A live editing session: one piece, draping, edited in place.
///
/// This is the whole surface a client gets. No solver type crosses it, which
/// is what lets the desktop app depend on the engine alone.
pub struct Session {
    slot: PieceSlot,
    contour: Vec<[f64; 2]>,
    drafted: Option<Drafted>,
    handle: SimHandle,
    generation: u64,
    revision: u64,
    /// How long the last recompile took, for the status bar.
    pub last_derive_ms: f64,
}

impl Session {
    /// The demo bodice, draping over the avatar on its own thread.
    pub fn demo_bodice() -> Session {
        let contour = demo::bodice_contour();
        let pipeline = demo::pipeline(&contour);
        let state = demo::drop_state(&pipeline);
        Session::spawn(PieceSlot::new(pipeline, 0), contour, None, state)
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
        if let [defect, ..] = draft.defects(piece) {
            return Err(SessionError::Defective {
                piece,
                defect: defect.clone(),
            });
        }
        let contour = draft.outline(piece).to_vec();
        let (samples, max_area) = couture::for_contour(&contour);
        let pipeline = ShapePipeline::build(&contour, samples, max_area)?;
        let state = couture::drop_state(&pipeline, couture::DROP_HEIGHT);
        let slot = PieceSlot::new(pipeline, draft.topology(piece));
        let drafted = Drafted { draft, piece };
        Ok(Session::spawn(slot, contour, Some(drafted), state))
    }

    /// The document this session edits, when it was opened from one.
    pub fn draft(&self) -> Option<&Draft> {
        self.drafted.as_ref().map(|held| &held.draft)
    }

    /// The piece this session drapes, when it was opened from a document.
    pub fn piece(&self) -> Option<PieceKey> {
        self.drafted.as_ref().map(|held| held.piece)
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

    /// Mesh triangles, indexing the snapshot's positions.
    pub fn triangles(&self) -> &[u32] {
        &self.slot.pipeline().tris
    }

    /// Mesh vertex count, for sizing render buffers.
    pub fn n_vertices(&self) -> usize {
        self.slot.pipeline().pos2d.len()
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

    /// Starts the sim thread on a freshly meshed piece.
    fn spawn(
        slot: PieceSlot,
        contour: Vec<[f64; 2]>,
        drafted: Option<Drafted>,
        state: toile_sim::xpbd::State,
    ) -> Session {
        let cons = slot.pipeline().constraints(COMPLIANCE);
        let handle = sync::spawn(
            state,
            cons,
            demo::avatar_sdf(),
            slot.pipeline().tris.clone(),
            DT,
            SUBSTEPS_PER_TICK,
        );
        Session {
            slot,
            contour,
            drafted,
            handle,
            generation: 0,
            revision: 0,
            last_derive_ms: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draft::{Axis, Binding, Command, block};

    /// The mesh is built at a topology count, and a shape edit that arrives
    /// against another one has to say so rather than warm-start across it.
    #[test]
    fn a_stale_generation_is_an_error_not_a_warm_start() {
        let mut session = Session::from_doc(block::trouser_front()).expect("the block drapes");
        let piece = session.piece().expect("the session has a document");
        session.slot.set_topology(7);
        let node = session
            .draft()
            .expect("the session has a document")
            .points_cm(piece)[1]
            .0;
        let moved = session.edit(Command::SetBinding {
            point: node,
            axis: Axis::X,
            to: Binding::literal(23.0),
        });
        assert_eq!(
            moved,
            Err(SessionError::TopologyMismatch {
                piece,
                expected: 7,
                got: 0
            })
        );
    }
}
