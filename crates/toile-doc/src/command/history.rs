use std::collections::BTreeSet;

use super::apply::Naming;
use crate::{Applied, Coalesced, Command, Doc, DocError, PieceKey};

/// The undo stack: one gesture, one entry.
///
/// A drag emits a command per frame and leaves a single entry behind, because
/// every command applied between `begin` and `end` folds into the entry the
/// gesture opened. The entry holds both directions — the commands that make
/// the edit and the commands that unmake it — so undo and redo are each one
/// step, and neither is reconstructed by inverting the other.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct History {
    done: Vec<Entry>,
    undone: Vec<Entry>,
    open: Option<Entry>,
}

/// One gesture's worth of edits, in both directions.
///
/// `inverse[i]` unmakes `forward[i]`, so undo walks the inverses backwards
/// and redo walks the forwards in order.
#[derive(Debug, Clone, PartialEq)]
struct Entry {
    label: &'static str,
    forward: Vec<Command>,
    inverse: Vec<Command>,
}

impl History {
    /// A history with nothing in it.
    pub fn new() -> History {
        History::default()
    }

    /// Opens a gesture named `label`; every edit until `end` folds into it.
    ///
    /// A gesture left open is closed here, so an interface that forgets an
    /// `end` loses the grouping and never the edits.
    pub fn begin(&mut self, label: &'static str) {
        self.end();
        self.open = Some(Entry::new(label));
    }

    /// Applies `command` and records it in the open gesture.
    ///
    /// An edit made with no gesture open is an entry of its own, unnamed. A
    /// recorded edit drops whatever redo was holding: the branch it belonged
    /// to is not the document's history any more.
    ///
    /// # Errors
    /// Whatever the command itself returns. A command that failed changed
    /// nothing and is not recorded.
    pub fn edit(&mut self, doc: &mut Doc, command: Command) -> Result<Applied, DocError> {
        let applied = command.clone().apply(doc)?;
        self.undone.clear();
        match &mut self.open {
            Some(entry) => entry.fold(command, applied.inverse.clone()),
            None => self
                .done
                .push(Entry::once(command, applied.inverse.clone())),
        }
        Ok(applied)
    }

    /// Closes the open gesture. A gesture that changed nothing leaves no entry.
    pub fn end(&mut self) {
        if let Some(entry) = self.open.take()
            && !entry.forward.is_empty()
        {
            self.done.push(entry);
        }
    }

    /// Takes back the last entry and names the pieces it changed, in key order.
    ///
    /// An open gesture is closed and undone whole. With nothing to take back
    /// the document is left alone and no piece is named.
    ///
    /// # Errors
    /// Whatever the inverse commands return. A failed undo puts back what it
    /// had already undone and keeps the entry, so the document and the stack
    /// never disagree.
    pub fn undo(&mut self, doc: &mut Doc) -> Result<Vec<PieceKey>, DocError> {
        let Some((entry, touched)) = self.take_back(doc)? else {
            return Ok(Vec::new());
        };
        self.undone.push(entry);
        Ok(touched)
    }

    /// Takes the last entry back and throws it away.
    ///
    /// This is the way out of a gesture rather than a step through the
    /// history: what the user refused mid-gesture — an aborted drag, a formula
    /// they chose to respect — leaves nothing for redo to bring back. With
    /// nothing to take back the document is left alone.
    ///
    /// # Errors
    /// The same as `undo`.
    pub fn cancel(&mut self, doc: &mut Doc) -> Result<Vec<PieceKey>, DocError> {
        Ok(self
            .take_back(doc)?
            .map_or_else(Vec::new, |(_, touched)| touched))
    }

    /// Closes the open gesture and unmakes the last entry, handing it over.
    ///
    /// The entry comes back with its forward commands refreshed from what the
    /// document produced, ready for a caller that means to keep it.
    fn take_back(&mut self, doc: &mut Doc) -> Result<Option<(Entry, Vec<PieceKey>)>, DocError> {
        self.end();
        let Some(mut entry) = self.done.pop() else {
            return Ok(None);
        };
        let backwards: Vec<Command> = entry.inverse.iter().rev().cloned().collect();
        match apply_all(doc, backwards) {
            Ok((produced, touched)) => {
                entry.forward = produced.into_iter().rev().collect();
                Ok(Some((entry, touched)))
            }
            Err(error) => {
                self.done.push(entry);
                Err(error)
            }
        }
    }

    /// Puts back the last entry undone and names the pieces it changed.
    ///
    /// Redo replays the forward commands. They are refreshed on every undo
    /// from what the document hands back, which is how redoing a deletion
    /// takes away the very key the undo restored.
    ///
    /// # Errors
    /// Whatever the forward commands return, with the same rollback as `undo`.
    pub fn redo(&mut self, doc: &mut Doc) -> Result<Vec<PieceKey>, DocError> {
        self.end();
        let Some(mut entry) = self.undone.pop() else {
            return Ok(Vec::new());
        };
        match apply_all(doc, entry.forward.clone()) {
            Ok((produced, touched)) => {
                entry.inverse = produced;
                self.done.push(entry);
                Ok(touched)
            }
            Err(error) => {
                self.undone.push(entry);
                Err(error)
            }
        }
    }

    /// The name of what undo would take back, for the status bar.
    pub fn undo_label(&self) -> Option<&str> {
        self.pending()
            .or_else(|| self.done.last())
            .map(|entry| entry.label)
    }

    /// The name of what redo would put back.
    pub fn redo_label(&self) -> Option<&str> {
        self.undone.last().map(|entry| entry.label)
    }

    /// How many entries undo can take back, an open gesture included.
    pub fn depth(&self) -> usize {
        self.done.len() + usize::from(self.pending().is_some())
    }

    /// How many entries redo can put back.
    pub fn redo_depth(&self) -> usize {
        self.undone.len()
    }

    /// The open gesture, once it has an edit in it.
    fn pending(&self) -> Option<&Entry> {
        self.open.as_ref().filter(|entry| !entry.forward.is_empty())
    }
}

impl Entry {
    fn new(label: &'static str) -> Entry {
        Entry {
            label,
            forward: Vec::new(),
            inverse: Vec::new(),
        }
    }

    fn once(command: Command, inverse: Command) -> Entry {
        Entry {
            label: "",
            forward: vec![command],
            inverse: vec![inverse],
        }
    }

    /// Records `command`, folding it onto an earlier write of the same field.
    fn fold(&mut self, command: Command, inverse: Command) {
        let earlier = self
            .forward
            .iter()
            .rposition(|before| command.coalesce_onto(before) == Coalesced::Replaces);
        // A fold keeps the first inverse: undo goes back to before the
        // gesture, not to the frame before the last one.
        if let Some(index) = earlier {
            self.forward[index] = command;
        } else {
            self.forward.push(command);
            self.inverse.push(inverse);
        }
    }
}

/// Replays every command in order, undoing them all again if one fails.
///
/// Half an undo would leave the document in a state no entry describes, so a
/// failure has to leave nothing behind. The names are replayed rather than
/// checked: an entry is one transaction, and a step of it may pass through a
/// collision the state on either side of it does not have.
fn apply_all(
    doc: &mut Doc,
    commands: Vec<Command>,
) -> Result<(Vec<Command>, Vec<PieceKey>), DocError> {
    let mut produced = Vec::with_capacity(commands.len());
    let mut touched = BTreeSet::new();
    for command in commands {
        match command.apply_as(doc, Naming::Restoring) {
            Ok(applied) => {
                touched.extend(applied.touched);
                produced.push(applied.inverse);
            }
            Err(error) => {
                rollback(doc, produced);
                return Err(error);
            }
        }
    }
    Ok((produced, touched.into_iter().collect()))
}

/// Puts back what `apply_all` had already applied, newest first.
fn rollback(doc: &mut Doc, produced: Vec<Command>) {
    for command in produced.into_iter().rev() {
        command
            .apply_as(doc, Naming::Restoring)
            .expect("the inverse of an edit just applied fits the state that edit made");
    }
}
