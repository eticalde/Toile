use std::time::Instant;

use super::remesh::{Job, Remeshed, Remesher};
use super::{Session, SessionError};

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
    /// `SessionError::Mesh` when the mesher refused the contour, in which case
    /// the piece keeps the mesh it had.
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

    /// Re-derives the drafted piece from its current geometry.
    ///
    /// A piece that has stopped resolving keeps the mesh it had: the viewer
    /// goes on showing the last good drape while the formula is fixed. So does
    /// one whose mesh is being rebuilt — the contour it would derive belongs
    /// to a topology this mesh does not have, and the rebuild picks the edit
    /// up when it lands.
    pub(super) fn rederive(&mut self) -> Result<(), SessionError> {
        if self.remeshing() {
            self.moved_while_meshing = true;
            return Ok(());
        }
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

    /// Sends the drafted piece to the mesher, and returns.
    ///
    /// Nothing on this thread waits for the answer. The solver keeps
    /// integrating the mesh it has, the table already draws the new line, and
    /// the two meet again in [`Session::poll_remesh`].
    pub(super) fn remesh(&mut self) {
        let Some(drafted) = self.drafted.as_ref() else {
            return;
        };
        let piece = drafted.piece;
        let contour = drafted.draft.outline(piece).to_vec();
        if contour.is_empty() {
            return;
        }
        // Every rebuild already sitting in the queue answers a contour this
        // edit has just replaced, so it would be dropped on the way in anyway.
        // Dropping it here is what keeps a burst of topology edits from piling
        // whole meshes up in the channel.
        while self
            .remesher
            .as_mut()
            .and_then(Remesher::try_take)
            .is_some()
        {}
        let job = Job {
            piece,
            topology: drafted.draft.topology(piece),
            contour,
            old_pos2d: self.slot.pipeline().pos2d.clone(),
            old_tris: self.slot.pipeline().tris.clone(),
        };
        self.remesher.get_or_insert_with(Remesher::spawn).send(job);
    }

    /// Puts a finished rebuild on the table and hands the drape to the solver.
    fn install(&mut self, done: Remeshed) -> Result<bool, SessionError> {
        let Some(drafted) = self.drafted.as_ref() else {
            return Ok(false);
        };
        // A rebuild the document has already moved past. The edit that
        // superseded it queued a rebuild of its own, and that one is the
        // answer; taking this one would mesh the piece as it no longer is.
        if done.piece != drafted.piece || done.topology != drafted.draft.topology(done.piece) {
            return Ok(false);
        }
        self.last_remesh_ms = done.ms;
        let built = match done.built {
            Ok(built) => built,
            Err(why) => {
                // The piece keeps the mesh it had, and the drag that was
                // waiting for a mesh that never came goes with it: the error
                // is what the person has to see, not a later mismatch.
                self.moved_while_meshing = false;
                return Err(why.into());
            }
        };
        self.slot.swap_in(built.pipeline, done.topology);
        self.contour = built.contour;
        self.generation += 1;
        self.mesh_generation = self.generation;
        self.handle.send_swap(self.generation, built.swap);
        // Whatever was dragged while the mesher worked was never derived. The
        // mesh it belongs to exists now, so it goes out as a shape edit.
        if std::mem::take(&mut self.moved_while_meshing) {
            self.rederive()?;
        }
        Ok(true)
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
