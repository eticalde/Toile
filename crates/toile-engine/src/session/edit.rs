use std::time::Instant;

use super::{Session, SessionError};
use crate::draft::{Command, Recompile};

impl Session {
    /// Applies an edit and recompiles whatever it touched.
    ///
    /// # Errors
    /// `SessionError` when the session has no document, when the document
    /// refuses the command, or when the edit changes a topology the session
    /// cannot mesh again yet.
    pub fn edit(&mut self, command: Command) -> Result<(), SessionError> {
        let drafted = self.drafted.as_mut().ok_or(SessionError::NoDocument)?;
        let what = drafted.draft.edit(command)?;
        self.revision += 1;
        self.recompile(what)
    }

    /// Opens a gesture: every edit until `end_gesture` is one undo entry.
    ///
    /// A session with no document has no history, so the call is a no-op
    /// rather than an error: the caller is bracketing a drag, not editing.
    pub fn begin_gesture(&mut self, label: &'static str) {
        if let Some(drafted) = self.drafted.as_mut() {
            drafted.draft.begin_gesture(label);
        }
    }

    /// Closes the open gesture. One that edited nothing leaves no entry.
    pub fn end_gesture(&mut self) {
        if let Some(drafted) = self.drafted.as_mut() {
            drafted.draft.end_gesture();
        }
    }

    /// Takes back the last entry and re-drapes whatever it changed.
    ///
    /// # Errors
    /// `SessionError` when the session has no document, when the document
    /// refuses an inverse, or when the step crosses a topology the session
    /// cannot mesh again yet.
    pub fn undo(&mut self) -> Result<(), SessionError> {
        let drafted = self.drafted.as_mut().ok_or(SessionError::NoDocument)?;
        let what = drafted.draft.undo()?;
        self.revision += 1;
        self.recompile(what)
    }

    /// Drops the open gesture and re-drapes whatever it had changed.
    ///
    /// The refused edits leave nothing for redo: this is the way out of a
    /// gesture, not a step through the history.
    ///
    /// # Errors
    /// The same as `undo`.
    pub fn cancel_gesture(&mut self) -> Result<(), SessionError> {
        let drafted = self.drafted.as_mut().ok_or(SessionError::NoDocument)?;
        let what = drafted.draft.cancel_gesture()?;
        self.revision += 1;
        self.recompile(what)
    }

    /// Puts the last entry undone back, and re-drapes it.
    ///
    /// # Errors
    /// The same as `undo`.
    pub fn redo(&mut self) -> Result<(), SessionError> {
        let drafted = self.drafted.as_mut().ok_or(SessionError::NoDocument)?;
        let what = drafted.draft.redo()?;
        self.revision += 1;
        self.recompile(what)
    }

    /// What undo would take back, named for the status bar.
    pub fn undo_label(&self) -> Option<&str> {
        self.drafted.as_ref()?.draft.undo_label()
    }

    /// What redo would put back.
    pub fn redo_label(&self) -> Option<&str> {
        self.drafted.as_ref()?.draft.redo_label()
    }

    /// Whether there is anything to take back.
    pub fn can_undo(&self) -> bool {
        self.drafted
            .as_ref()
            .is_some_and(|held| held.draft.undo_depth() > 0)
    }

    /// Whether there is anything to put back.
    pub fn can_redo(&self) -> bool {
        self.drafted
            .as_ref()
            .is_some_and(|held| held.draft.redo_depth() > 0)
    }

    /// Pays whatever the last change to the document cost the drape.
    fn recompile(&mut self, what: Recompile) -> Result<(), SessionError> {
        let Some(piece) = self.piece() else {
            return Ok(());
        };
        match what {
            Recompile::Shape(pieces) if pieces.contains(&piece) => self.rederive(),
            Recompile::Nothing | Recompile::Shape(_) => Ok(()),
            Recompile::Topology(_) => Err(SessionError::NoRemesher),
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
