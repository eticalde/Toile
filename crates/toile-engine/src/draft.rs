mod edit;
mod env;
mod order;
mod resolve;

use std::collections::BTreeMap;

pub use edit::Recompile;
pub use env::{Env, EnvError};
pub use resolve::{Defect, Resolved, to_document, to_metres};
// The one door between the document and the interface. The desktop app
// depends on this crate and on nothing else of Toile's, so a type reaches it
// only by being written on this list, one reviewable line at a time.
pub use toile_doc::block;
pub use toile_doc::formula::{EvalError, Formula, Lookup, SyntaxError};
pub use toile_doc::{
    Applied, Axis, Binding, ChangeClass, Command, ContourNode, Doc, DocError, EdgeAnchor,
    EdgeRange, Grain, History, Identity, MannequinKey, MeasureSet, NotchCount, Piece, PieceKey,
    Point, PointKey, SeamKey, Segment, Variable, VariableKey, Winding,
};
pub use toile_geom::validate::ContourFault;

/// What stops the document from being drafted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DraftError {
    /// The document refused the command.
    #[error(transparent)]
    Doc(#[from] DocError),
    /// The names the formulas read could not be given values.
    #[error(transparent)]
    Env(#[from] EnvError),
}

/// One piece as the draft holds it: its last good geometry, whatever is wrong
/// with it now, and how many times its topology has moved.
#[derive(Debug, Clone, Default, PartialEq)]
struct Compiled {
    good: Resolved,
    defects: Vec<Defect>,
    topology: u64,
}

/// A document and the geometry it currently resolves to.
///
/// Two truths per coordinate live here, and only one of them is written: the
/// binding is the document's, the resolved number is this module's, and it is
/// never written back. A piece that stops resolving keeps its last good
/// geometry and gains a defect, so the drawing and the drape hold still while
/// the person fixes the formula.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    doc: Doc,
    env: Env,
    history: History,
    points: BTreeMap<PointKey, [f64; 2]>,
    pieces: BTreeMap<PieceKey, Compiled>,
}

impl Draft {
    /// Resolves a document into geometry.
    ///
    /// # Errors
    /// `DraftError::Env` when the document's names cannot be given values at
    /// all: no body to resolve against, a variable named after a measurement,
    /// or variables that depend on each other.
    pub fn from_doc(doc: Doc) -> Result<Draft, DraftError> {
        let mut draft = Draft {
            doc,
            env: Env::default(),
            history: History::new(),
            points: BTreeMap::new(),
            pieces: BTreeMap::new(),
        };
        draft.resolve_all()?;
        Ok(draft)
    }

    /// The document itself.
    pub fn doc(&self) -> &Doc {
        &self.doc
    }

    /// What every name in the formulas is worth, in centimetres.
    pub fn env(&self) -> &Env {
        &self.env
    }

    /// A piece's contour in metres with y upward: what the engine meshes.
    ///
    /// Empty for a piece that has never resolved.
    pub fn outline(&self, piece: PieceKey) -> &[[f64; 2]] {
        self.pieces
            .get(&piece)
            .map_or(&[], |held| held.good.outline.as_slice())
    }

    /// A piece's nodes in centimetres with y downward: what the table draws.
    pub fn points_cm(&self, piece: PieceKey) -> &[(PointKey, [f64; 2])] {
        self.pieces
            .get(&piece)
            .map_or(&[], |held| held.good.points.as_slice())
    }

    /// Where a point resolved to, in centimetres.
    pub fn resolved(&self, point: PointKey) -> Option<[f64; 2]> {
        self.points.get(&point).copied()
    }

    /// A piece's perimeter in centimetres.
    pub fn perimeter_cm(&self, piece: PieceKey) -> f64 {
        self.pieces
            .get(&piece)
            .and_then(|held| held.good.cum.last().copied())
            .unwrap_or_default()
    }

    /// The length in centimetres of the walk from one node to another, the way
    /// the contour runs.
    ///
    /// Zero when either node is not on the piece, which is what a length nobody
    /// can point at is worth.
    pub fn run_length_cm(&self, piece: PieceKey, from: PointKey, to: PointKey) -> f64 {
        let Some(held) = self.pieces.get(&piece) else {
            return 0.0;
        };
        let at = |key: PointKey| held.good.points.iter().position(|&(p, _)| p == key);
        let (Some(from), Some(to)) = (at(from), at(to)) else {
            return 0.0;
        };
        let cum = &held.good.cum;
        if to >= from {
            cum[to] - cum[from]
        } else {
            cum[cum.len() - 1] - cum[from] + cum[to]
        }
    }

    /// What is wrong with a piece right now, in contour order.
    pub fn defects(&self, piece: PieceKey) -> &[Defect] {
        self.pieces
            .get(&piece)
            .map_or(&[], |held| held.defects.as_slice())
    }

    /// How many times a piece's topology has changed under this draft.
    ///
    /// Derivation state: it is not saved, not undone, and not part of the
    /// document. A mesh built at one count cannot be warm-started at another.
    pub fn topology(&self, piece: PieceKey) -> u64 {
        self.pieces.get(&piece).map_or(0, |held| held.topology)
    }

    /// Applies a command, records it, and says what has to be recompiled.
    ///
    /// # Errors
    /// `DraftError::Doc` when the document refuses the command, and
    /// `DraftError::Env` when the edit would leave the formulas unresolvable —
    /// in which case nothing is kept, so neither the document nor the undo
    /// stack ever holds a state that cannot be drawn.
    ///
    /// # Panics
    /// If taking the rehearsal back out fails, which would mean a command's
    /// own inverse was refused by the document it had just been applied to.
    pub fn edit(&mut self, command: Command) -> Result<Recompile, DraftError> {
        // Rehearsed before it is recorded. An edit that leaves the formulas
        // unresolvable has to vanish whole, and an entry already folded into
        // an open gesture cannot be taken back on its own.
        let rehearsal = command.clone().apply(&mut self.doc)?;
        let refused = env::build(&self.doc).err();
        rehearsal
            .inverse
            .apply(&mut self.doc)
            .expect("the inverse of an edit that just applied");
        if let Some(broken) = refused {
            return Err(broken.into());
        }
        let applied = self.history.edit(&mut self.doc, command)?;
        let what = edit::recompile(&applied);
        self.resolve_all()?;
        if let Recompile::Topology(pieces) = &what {
            for &piece in pieces {
                self.pieces.entry(piece).or_default().topology += 1;
            }
        }
        Ok(what)
    }

    /// Opens a gesture: everything edited until `end_gesture` is one entry.
    pub fn begin_gesture(&mut self, label: &'static str) {
        self.history.begin(label);
    }

    /// Closes the open gesture. One that edited nothing leaves no entry.
    pub fn end_gesture(&mut self) {
        self.history.end();
    }

    /// Takes back the last entry and says what has to be recompiled.
    ///
    /// # Errors
    /// `DraftError` when the document refuses an inverse, or when the state it
    /// goes back to no longer resolves.
    pub fn undo(&mut self) -> Result<Recompile, DraftError> {
        let touched = self.history.undo(&mut self.doc)?;
        self.recover(touched)
    }

    /// Drops the open gesture and says what has to be recompiled.
    ///
    /// This is how a gesture is refused rather than stepped back through: what
    /// the user declined leaves nothing for redo to bring back.
    ///
    /// # Errors
    /// The same as `undo`.
    pub fn cancel_gesture(&mut self) -> Result<Recompile, DraftError> {
        let touched = self.history.cancel(&mut self.doc)?;
        self.recover(touched)
    }

    /// Puts the last entry undone back, and says what has to be recompiled.
    ///
    /// # Errors
    /// The same as `undo`.
    pub fn redo(&mut self) -> Result<Recompile, DraftError> {
        let touched = self.history.redo(&mut self.doc)?;
        self.recover(touched)
    }

    /// What undo would take back, for the status bar.
    pub fn undo_label(&self) -> Option<&str> {
        self.history.undo_label()
    }

    /// What redo would put back.
    pub fn redo_label(&self) -> Option<&str> {
        self.history.redo_label()
    }

    /// How many entries undo can take back, an open gesture included.
    pub fn undo_depth(&self) -> usize {
        self.history.depth()
    }

    /// How many entries redo can put back.
    pub fn redo_depth(&self) -> usize {
        self.history.redo_depth()
    }

    /// Re-resolves after the history moved, and prices what the move cost.
    ///
    /// The class is read off the geometry rather than remembered: a step that
    /// changed how long a contour is has to be meshed again, whatever the
    /// commands inside the entry were.
    fn recover(&mut self, touched: Vec<PieceKey>) -> Result<Recompile, DraftError> {
        let before: Vec<usize> = touched
            .iter()
            .map(|&piece| self.outline(piece).len())
            .collect();
        self.resolve_all()?;
        let remeshed = touched
            .iter()
            .zip(&before)
            .any(|(&piece, &was)| self.outline(piece).len() != was);
        if !remeshed {
            return Ok(Recompile::Shape(touched));
        }
        for &piece in &touched {
            self.pieces.entry(piece).or_default().topology += 1;
        }
        Ok(Recompile::Topology(touched))
    }

    /// Rebuilds the environment and every piece's geometry.
    ///
    /// A piece that fails keeps the geometry it last had: the drawing and the
    /// drape stay put, and the defect is what changes.
    fn resolve_all(&mut self) -> Result<(), DraftError> {
        self.env = env::build(&self.doc)?;
        let (good, broken) = resolve::points(&self.doc, &self.env);
        for (key, piece) in self.doc.pieces.iter() {
            let held = self.pieces.entry(key).or_default();
            match resolve::piece(piece, &good, &broken) {
                Ok(fresh) => {
                    held.good = fresh;
                    held.defects.clear();
                }
                Err(defects) => held.defects = defects,
            }
        }
        self.pieces
            .retain(|key, _| self.doc.pieces.get(*key).is_some());
        self.points = good;
        Ok(())
    }
}
