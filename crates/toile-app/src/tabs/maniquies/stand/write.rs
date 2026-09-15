use toile_engine::draft::{BodyShape, Command, Identity, MeasureSet};
use toile_engine::session::{Session, SessionError};

use super::Stand;

const RENAME: &str = "renombrar maniquí";
const ADD: &str = "nuevo maniquí";

/// A control of the tab, told apart so a gesture knows whose it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Control {
    /// One measurement's slider, by catalogue name.
    Measure(String),
    /// The phenotype sliders, which all write the one shape.
    Shape,
}

impl Control {
    /// The name the history keeps the gesture under.
    fn label(&self) -> &'static str {
        match self {
            Control::Measure(_) => "editar medida",
            Control::Shape => "dar forma al maniquí",
        }
    }
}

impl Stand {
    /// Takes a control in hand, opening its gesture unless it is the one
    /// already open.
    ///
    /// Every value it writes until the gesture closes folds into one undo
    /// entry: letting go is the deliberate act, and no drag frame before it is.
    pub fn grip(&mut self, session: &mut Session, control: Control) {
        if self.held.as_ref() == Some(&control) {
            return;
        }
        self.release(session);
        session.begin_gesture(control.label());
        self.held = Some(control);
    }

    /// Says a pointer is still down on `control`, which keeps its gesture open
    /// through [`Stand::settle`].
    pub fn keep(&mut self, control: &Control) {
        if self.held.as_ref() == Some(control) {
            self.holding = true;
        }
    }

    /// Ends the frame for the control in hand: its gesture closes unless a
    /// pointer was kept on it. Answers whether one closed.
    ///
    /// A click, a typed value or an arrow key opens and closes its gesture in
    /// the same frame; a drag keeps it open until the pointer comes off.
    pub fn settle(&mut self, session: &mut Session) -> bool {
        let holding = std::mem::take(&mut self.holding);
        if holding || self.held.is_none() {
            return false;
        }
        self.release(session);
        true
    }

    pub(super) fn release(&mut self, session: &mut Session) {
        if self.held.take().is_some() {
            session.end_gesture();
        }
    }

    /// Undoes or redoes the product's last entry from this tab, closing any
    /// gesture still open first so the entry it steps over is a whole one.
    pub fn step(&mut self, session: &mut Session, redo: bool) {
        self.release(session);
        let can = if redo {
            session.can_redo()
        } else {
            session.can_undo()
        };
        if !can {
            return;
        }
        let answer = if redo { session.redo() } else { session.undo() };
        self.answer(session, answer);
    }

    /// Writes one measurement the body carries.
    ///
    /// A value the body already holds writes nothing, so a press that moves
    /// nothing leaves no entry behind.
    pub fn set_measure(&mut self, session: &mut Session, name: &str, cm: f64) {
        let stored = self.body(session).get(name).map(f64::to_bits);
        if !cm.is_finite() || stored == Some(cm.to_bits()) {
            return;
        }
        let Some(draft) = session.draft() else {
            self.loose.values.insert(name.to_owned(), cm);
            return;
        };
        let command = Command::SetMeasure {
            mannequin: draft.doc().resolve_with,
            name: name.to_owned(),
            to: cm,
        };
        let answer = session.edit(command);
        self.answer(session, answer);
    }

    /// Gives the body a shape.
    ///
    /// The shape it stands at writes nothing, even while that is only the
    /// default a body without one reads as: a product that stores no shape
    /// keeps its bytes until a control really moves.
    pub fn set_shape(&mut self, session: &mut Session, shape: BodyShape) {
        if self.shape(session) == shape {
            return;
        }
        let Some(draft) = session.draft() else {
            self.loose.phenotype = Some(shape);
            return;
        };
        let command = Command::SetPhenotype {
            mannequin: draft.doc().resolve_with,
            to: Some(shape),
        };
        let answer = session.edit(command);
        self.answer(session, answer);
    }

    /// Renames the body. An empty name, or the one it carries, writes nothing.
    pub fn rename(&mut self, session: &mut Session, to: &str) {
        let to = to.trim();
        if to.is_empty() || to == self.body(session).name {
            return;
        }
        self.release(session);
        let Some(draft) = session.draft() else {
            to.clone_into(&mut self.loose.name);
            return;
        };
        if Self::name_taken(session, to) {
            let why = format!("ya hay un maniquí llamado «{to}»");
            self.refusal = Some((why, session.revision()));
            return;
        }
        let command = Command::RenameMannequin {
            mannequin: draft.doc().resolve_with,
            to: to.to_owned(),
        };
        session.begin_gesture(RENAME);
        let answer = session.edit(command);
        session.end_gesture();
        self.answer(session, answer);
    }

    /// Puts a new body on the stand.
    ///
    /// With a product open it joins the product and becomes the body the
    /// pattern resolves with, in one undo entry, so a single undo takes back
    /// the choice and the body together. With none it takes the loose body's
    /// place.
    pub fn add(&mut self, session: &mut Session, set: MeasureSet) {
        self.add_as(session, set, ADD);
    }

    /// Puts a new body on the stand as `add` does, under the entry `label`.
    ///
    /// # Panics
    /// Never: a body the document has just taken is found under its name.
    pub(super) fn add_as(&mut self, session: &mut Session, set: MeasureSet, label: &'static str) {
        self.release(session);
        if session.draft().is_none() {
            self.loose = set;
            return;
        }
        let name = set.name.clone();
        session.begin_gesture(label);
        let added = session.edit(Command::AddMannequin {
            identity: Identity::New,
            mannequin: set,
        });
        let answer = match added {
            Ok(()) => {
                let mannequin = session
                    .draft()
                    .and_then(|draft| draft.doc().mannequin_named(&name))
                    .expect("the body just added carries the name it was added under");
                let chosen = session.edit(Command::ResolveWith { mannequin });
                // A body the pattern cannot resolve with is not left behind
                // half made. Cancelling is only safe here, with the addition
                // in the gesture: an empty one would take the entry before it.
                if chosen.is_err() {
                    let _ = session.cancel_gesture();
                }
                chosen
            }
            Err(why) => Err(why),
        };
        session.end_gesture();
        self.answer(session, answer);
    }

    /// Keeps what the session refused, or forgets it once an edit went
    /// through.
    pub(super) fn answer(&mut self, session: &Session, answer: Result<(), SessionError>) {
        self.refusal = answer
            .err()
            .map(|why| (why.to_string(), session.revision()));
    }
}
