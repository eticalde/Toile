mod error;
mod slot;

use std::sync::Arc;
use std::time::Instant;

pub use error::SessionError;
pub use slot::PieceSlot;

use crate::couture::{self, ShapePipeline};
use crate::demo;
use crate::draft::{self, Binding, Command, Doc, Draft, PieceKey, Recompile};
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

    /// Applies an edit and recompiles whatever it touched.
    ///
    /// # Errors
    /// `SessionError` when the session has no document, when the document
    /// refuses the command, or when the edit changes a topology the session
    /// cannot mesh again yet.
    pub fn edit(&mut self, command: Command) -> Result<(), SessionError> {
        let drafted = self.drafted.as_mut().ok_or(SessionError::NoDocument)?;
        let piece = drafted.piece;
        match drafted.draft.edit(command)? {
            Recompile::Shape(pieces) if pieces.contains(&piece) => self.rederive(),
            Recompile::Nothing | Recompile::Shape(_) => Ok(()),
            Recompile::Topology(_) => Err(SessionError::NoRemesher),
        }
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

    /// Moves a control point and hot-swaps the resulting rest state.
    ///
    /// The index is a position in the contour, which is what the viewer's 2D
    /// half still has to give; while every tract is a straight line that is
    /// the same thing as the node at that position, so a document-backed
    /// session writes the move as a command. Out-of-range indices and refused
    /// edits leave the drape alone.
    pub fn move_point(&mut self, index: usize, to: [f64; 2]) {
        if let Some(drafted) = &self.drafted {
            let Some(&(point, _)) = drafted.draft.points_cm(drafted.piece).get(index) else {
                return;
            };
            let [x, y] = draft::to_document(to);
            let moved = Command::MovePoint {
                point,
                to: [Binding::literal(x), Binding::literal(y)],
            };
            let _ = self.edit(moved);
            return;
        }
        if index >= self.contour.len() {
            return;
        }
        let mut contour = self.contour.clone();
        contour[index] = to;
        let _ = self.send(contour);
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
            last_derive_ms: 0.0,
        }
    }

    /// Re-derives the drafted piece from its current geometry.
    ///
    /// A piece that has stopped resolving keeps the mesh it had: the viewer
    /// goes on showing the last good drape while the formula is fixed.
    fn rederive(&mut self) -> Result<(), SessionError> {
        let Some(drafted) = self.drafted.as_ref() else {
            return Ok(());
        };
        let piece = drafted.piece;
        let topology = drafted.draft.topology(piece);
        let outline = drafted.draft.outline(piece).to_vec();
        if outline.is_empty() {
            return Ok(());
        }
        if topology != self.slot.topology() {
            return Err(SessionError::TopologyMismatch {
                piece,
                expected: self.slot.topology(),
                got: topology,
            });
        }
        self.send(outline)
    }

    /// Derives a contour into rest lengths and hands them to the sim thread.
    fn send(&mut self, contour: Vec<[f64; 2]>) -> Result<(), SessionError> {
        let t = Instant::now();
        let rests = self.slot.derive(&contour)?.to_vec();
        self.last_derive_ms = t.elapsed().as_secs_f64() * 1000.0;
        self.contour = contour;
        self.generation += 1;
        self.handle.send_rests(self.generation, rests);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draft::{Axis, block};

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
