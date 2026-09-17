use std::time::Instant;

use super::remesh::{Job, Remeshed, Remesher};
use super::{Session, SessionError};
use crate::couture::MeshSwap;
use crate::draft::PieceKey;

impl Session {
    /// Whether a rebuild is out with the mesher.
    ///
    /// The drape on the stand is the old mesh's until it comes back, and the
    /// piece on the table is already the new one: this is what the interface
    /// asks in order to say so.
    pub fn remeshing(&self) -> bool {
        self.remesher.as_ref().is_some_and(Remesher::busy)
    }

    /// Takes in whatever the mesher has finished. Does not block.
    ///
    /// The interface calls it once a frame. It answers whether a mesh actually
    /// changed hands, which is what asks for the frame that shows it.
    ///
    /// # Errors
    /// `SessionError::Mesh` when the mesher refused a contour, in which case
    /// that piece keeps the mesh it had.
    pub fn poll_remesh(&mut self) -> Result<bool, SessionError> {
        let mut landed = false;
        while let Some(done) = self.remesher.as_mut().and_then(Remesher::try_take) {
            landed |= self.install(done)?;
        }
        Ok(landed)
    }

    /// The same, waiting for every rebuild that is out.
    ///
    /// The interface never calls this: it is how a test or a benchmark asks
    /// for the wait the person would have had.
    ///
    /// # Errors
    /// The same as [`Session::poll_remesh`].
    pub fn wait_for_remesh(&mut self) -> Result<bool, SessionError> {
        let mut landed = false;
        while let Some(done) = self.remesher.as_mut().and_then(Remesher::take) {
            landed |= self.install(done)?;
        }
        Ok(landed)
    }

    /// Re-derives the pieces an edit reshaped, then hands the whole product's
    /// rest lengths to the solver once.
    ///
    /// A piece that has stopped resolving keeps the mesh it had: the viewer
    /// goes on showing the last good drape while the formula is fixed. So does
    /// one whose own mesh is being rebuilt — the contour it would derive
    /// belongs to a topology that mesh does not have, and the rebuild picks
    /// the edit up when it lands. A piece the edit did not name is not read at
    /// all, so its drape cannot move.
    ///
    /// # Errors
    /// `SessionError::TopologyMismatch` when a named piece's mesh was built
    /// from another set of nodes, and `SessionError::RestState` when its
    /// contour no longer matches the mesh built from it.
    pub(super) fn rederive(&mut self, pieces: &[PieceKey]) -> Result<(), SessionError> {
        let t = Instant::now();
        let mut derived = false;
        for &piece in pieces {
            let Some(at) = self.index_of(piece) else {
                continue;
            };
            if self.draping[at].rebuilds > 0 {
                self.draping[at].moved_while_meshing = true;
                continue;
            }
            let Some(draft) = self.draft.as_ref() else {
                return Ok(());
            };
            let topology = draft.topology(piece);
            let outline = draft.outline_m(piece).to_vec();
            if outline.is_empty() {
                continue;
            }
            let held = &mut self.draping[at];
            let expected = held.slot.topology();
            if topology != expected {
                return Err(SessionError::TopologyMismatch {
                    piece,
                    expected,
                    got: topology,
                });
            }
            held.slot.derive(&outline)?;
            held.contour = outline;
            derived = true;
        }
        if derived {
            self.last_derive_ms = t.elapsed().as_secs_f64() * 1000.0;
            self.send_rests();
        }
        Ok(())
    }

    /// Sends the pieces an edit re-shaped to the mesher, and returns.
    ///
    /// Nothing on this thread waits for the answer. The solver keeps
    /// integrating the meshes it has, the table already draws the new line,
    /// and the two meet again in [`Session::poll_remesh`].
    ///
    /// # Errors
    /// The same as [`Session::poll_remesh`], which this collects first.
    pub(super) fn remesh(&mut self, pieces: &[PieceKey]) -> Result<(), SessionError> {
        // Whatever the mesher has already finished comes in before more goes
        // out. A rebuild of a piece this edit touched is dropped on the way in
        // by its topology count, which is what keeps a burst of edits from
        // piling whole meshes up in the channel; a rebuild of any other piece
        // is an answer nobody else will give, and throwing it away would cost
        // that piece the drape it has.
        self.poll_remesh()?;
        for &piece in pieces {
            let Some(at) = self.index_of(piece) else {
                continue;
            };
            let Some(draft) = self.draft.as_ref() else {
                return Ok(());
            };
            let topology = draft.topology(piece);
            let contour = draft.outline_m(piece).to_vec();
            if contour.is_empty() {
                continue;
            }
            let pipeline = self.draping[at].slot.pipeline();
            let job = Job {
                piece,
                topology,
                contour,
                old_pos2d: pipeline.pos2d.clone(),
                old_tris: pipeline.tris.clone(),
            };
            self.remesher.get_or_insert_with(Remesher::spawn).send(job);
            self.draping[at].rebuilds += 1;
        }
        Ok(())
    }

    /// Puts a finished rebuild on the table and hands the drape to the solver.
    fn install(&mut self, done: Remeshed) -> Result<bool, SessionError> {
        let Some(at) = self.index_of(done.piece) else {
            return Ok(false);
        };
        self.draping[at].rebuilds = self.draping[at].rebuilds.saturating_sub(1);
        // A rebuild the document has already moved past. The edit that
        // superseded it queued a rebuild of its own, and that one is the
        // answer; taking this one would mesh the piece as it no longer is.
        let stale = self
            .draft
            .as_ref()
            .is_none_or(|draft| done.topology != draft.topology(done.piece));
        if stale {
            return Ok(false);
        }
        self.last_remesh_ms = done.ms;
        let built = match done.built {
            Ok(built) => built,
            Err(why) => {
                // The piece keeps the mesh it had, and the drag that was
                // waiting for a mesh that never came goes with it: the error
                // is what the person has to see, not a later mismatch.
                self.draping[at].moved_while_meshing = false;
                return Err(why.into());
            }
        };
        if self.handle.is_none() {
            return Ok(false);
        }
        // Where this piece stood before the rebuild. Its own block is the run
        // of the combined state the swap replaces; every piece after it moves
        // by whatever the rebuild changed the count by, and every piece before
        // it does not move at all.
        let base = self.offset(done.piece).unwrap_or_default();
        let replacing = self.draping[at].slot.pipeline().pos2d.len() as u32;
        self.draping[at].slot.swap_in(built.pipeline, done.topology);
        self.draping[at].contour = built.contour;
        self.relist();
        // Every seam of the product is paired again, not only this piece's.
        // The rebuild handed this piece a whole new set of vertices and moved
        // the base of every piece standing after it, so there is no seam whose
        // indices can be assumed to have survived it.
        let seams = self.resew();
        let swap = Box::new(MeshSwap {
            locator: built.locator,
            pos2d: self.draping[at].slot.pipeline().pos2d.clone(),
            tris: self.tris.clone(),
            cons: self.constraints(),
            seams,
            at: base,
            replacing,
        });
        self.generation += 1;
        self.mesh_generation = self.generation;
        if let Some(handle) = self.handle.as_ref() {
            handle.send_swap(self.generation, swap);
        }
        // Whatever was dragged while the mesher worked was never derived. The
        // mesh it belongs to exists now, so it goes out as a shape edit.
        if std::mem::take(&mut self.draping[at].moved_while_meshing) {
            self.rederive(&[done.piece])?;
        }
        Ok(true)
    }

    /// Hands the whole product's rest lengths, and what is sewn to what, to
    /// the sim thread.
    ///
    /// Every piece goes, not only the one that moved. The solver checks the
    /// count against the constraint set it holds, and that check is the whole
    /// of what tells rest lengths compiled for the meshes on the stand from
    /// ones left over from a set it has already swapped away. A piece nobody
    /// edited contributes the very numbers it already had, so the message says
    /// nothing new about it.
    ///
    /// The seams travel with them because a shape edit moves them too: an
    /// anchor is a node and a fraction of the tract leaving it, so a piece
    /// that changed shape reads its seams onto different boundary vertices
    /// than it did before.
    fn send_rests(&mut self) {
        if self.handle.is_none() {
            return;
        }
        let rests: Vec<f32> = self
            .draping
            .iter()
            .flat_map(|held| held.slot.pipeline().rests().iter().copied())
            .collect();
        let seams = self.resew();
        self.generation += 1;
        if let Some(handle) = self.handle.as_ref() {
            handle.send_rests(self.generation, rests, seams);
        }
    }
}
