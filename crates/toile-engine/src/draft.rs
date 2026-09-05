mod contour;
mod defect;
mod edit;
mod env;
mod history;
mod order;
mod resolve;

use std::collections::BTreeMap;

pub use defect::Defect;
pub use edit::Recompile;
pub use env::{Env, EnvError};
pub use resolve::{Resolved, to_document, to_metres};
// The one door between the document and the interface. The desktop app
// depends on this crate and on nothing else of Toile's, so a type reaches it
// only by being written on this list, one reviewable line at a time.
pub use toile_doc::block;
pub use toile_doc::formula::{EvalError, Formula, Lookup, SyntaxError};
pub use toile_doc::{
    Applied, Axis, Binding, ChangeClass, Command, ContourNode, Doc, DocError, EdgeAnchor,
    EdgeRange, Grain, Handle, Handles, History, Identity, MannequinKey, MeasureSet, NotchCount,
    Piece, PieceKey, Point, PointKey, SAMPLES, SeamKey, Segment, SegmentEdit, Variable,
    VariableKey, Winding,
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

    /// A piece's flattened contour in metres with y upward: what the engine
    /// meshes.
    ///
    /// Empty for a piece that has never resolved.
    pub fn outline(&self, piece: PieceKey) -> &[[f64; 2]] {
        self.pieces
            .get(&piece)
            .map_or(&[], |held| held.good.outline.as_slice())
    }

    /// The same flattened contour in centimetres with y downward: the line the
    /// table draws, curves and all.
    pub fn flat_cm(&self, piece: PieceKey) -> &[[f64; 2]] {
        self.pieces
            .get(&piece)
            .map_or(&[], |held| held.good.flat.as_slice())
    }

    /// A piece's nodes in centimetres with y downward: what the table selects.
    pub fn points_cm(&self, piece: PieceKey) -> &[(PointKey, [f64; 2])] {
        self.pieces
            .get(&piece)
            .map_or(&[], |held| held.good.points.as_slice())
    }

    /// Where each node opens in the flattened contour, in contour order.
    ///
    /// The tract leaving node `i` is `flat_cm[starts[i]..starts[i + 1]]`, and
    /// the last one closes on the first node again. The interface walks the
    /// drawn line tract by tract with this instead of flattening the document
    /// a second time, which is how the line it paints and the line it catches
    /// stay the same line.
    pub fn flat_starts(&self, piece: PieceKey) -> &[usize] {
        self.pieces
            .get(&piece)
            .map_or(&[], |held| held.good.starts.as_slice())
    }

    /// Where a point resolved to, in centimetres.
    pub fn resolved(&self, point: PointKey) -> Option<[f64; 2]> {
        self.points.get(&point).copied()
    }

    /// A piece's perimeter in centimetres, measured along the flattening.
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
    /// If taking a refused edit back out fails, which would mean a command's
    /// own inverse was refused by the document it had just been applied to.
    pub fn edit(&mut self, command: Command) -> Result<Recompile, DraftError> {
        // Applied once and recorded from what that produced. An edit that
        // leaves the formulas unresolvable has to vanish whole — an entry
        // already folded into an open gesture cannot be taken back on its own
        // — but applying it twice to find that out would issue two sets of
        // keys for an edit that creates entities, and the first set would
        // stay burned.
        let applied = command.clone().apply(&mut self.doc)?;
        if let Some(broken) = env::build(&self.doc).err() {
            applied
                .inverse
                .clone()
                .apply(&mut self.doc)
                .expect("the inverse of an edit that just applied");
            return Err(broken.into());
        }
        self.history.record(command, applied.inverse.clone());
        let what = edit::recompile(&applied);
        self.resolve_all()?;
        if let Recompile::Topology(pieces) = &what {
            for &piece in pieces {
                self.pieces.entry(piece).or_default().topology += 1;
            }
        }
        Ok(what)
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
