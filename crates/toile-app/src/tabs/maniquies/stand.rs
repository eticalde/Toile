mod person;
mod write;

use std::collections::BTreeMap;

pub use person::{Saved, Wrote};
use toile_engine::body::{self, AnnySolve, Tape};
use toile_engine::draft::{BodyMesh, BodyShape, MeasureSet};
use toile_engine::session::Session;
pub use write::Control;

use super::identity::NewManiqui;
use crate::tabs::Kept;

/// What a new body is called until someone names it.
pub const NEW_NAME: &str = "Maniquí";

/// Every input a solve reads, and nothing else, so a rename costs no rebuild.
#[derive(Debug, Clone, PartialEq)]
struct Basis {
    values: BTreeMap<String, f64>,
    shape: BodyShape,
}

/// The tab's hold on the body it shows, apart from the GPU view that paints
/// it, so everything it decides can be run without a window.
///
/// A product's body is never copied in here: it is read from the document on
/// every frame and written back through the session. The one body this holds
/// is the loose one, shaped while no product is open.
pub struct Stand {
    /// The body the tab shapes while no product is open. Nothing saves it.
    loose: MeasureSet,
    /// The control whose gesture is open.
    held: Option<Control>,
    /// Whether a pointer was still down on that control this frame.
    holding: bool,
    /// What the session last refused, and the revision it was refused at.
    refusal: Option<(String, u64)>,
    /// What the mesh on the view was solved from.
    built: Option<Basis>,
    /// The name being typed, while the name field has the focus.
    pub naming: Option<String>,
    /// The last solve: what the body measures at every row.
    pub solved: Option<AnnySolve>,
    /// The stature the last mesh measured, in centimetres, which Anny's own
    /// `height` input is not.
    pub stature_cm: Option<f32>,
    /// The catalogue name whose tape lies on the body: the row under the
    /// pointer, the slider in hand or the box with the focus.
    ///
    /// Nothing outlives the hand that pointed at it. The measures panel
    /// writes this every frame from the row that answered, so a pointer
    /// leaving the panel takes the tape off the body with it.
    pub highlight: Option<String>,
    /// The name the tape on the view was laid for.
    taped: Option<String>,
    /// Whether the body was solved again since the tape was laid, which
    /// moves the tape even when the name stayed.
    resolved: bool,
    /// The new-mannequin dialog's own fields, while it is open.
    pub new_dialog: Option<NewManiqui>,
}

impl Default for Stand {
    fn default() -> Self {
        let mut loose = body::default_measures();
        NEW_NAME.clone_into(&mut loose.name);
        Self {
            loose,
            held: None,
            holding: false,
            refusal: None,
            built: None,
            naming: None,
            solved: None,
            stature_cm: None,
            highlight: None,
            taped: None,
            resolved: true,
            new_dialog: None,
        }
    }
}

impl Stand {
    /// Drops everything that belonged to the product that was open: an open
    /// gesture, a half-typed name, the last refusal and the solve.
    ///
    /// None of it may outlive that product. A slider still in hand would keep
    /// writing into the next one, and a pending name would rename its body.
    pub fn forget(&mut self) {
        self.held = None;
        self.holding = false;
        self.refusal = None;
        self.built = None;
        self.naming = None;
        self.new_dialog = None;
    }

    /// The body the tab shows and edits: the one the product's pattern
    /// resolves against, or the loose one while no product is open.
    ///
    /// # Panics
    /// Never: a draft only exists over a document whose formulas found the
    /// body they resolve against.
    pub fn body<'a>(&'a self, session: &'a Session) -> &'a MeasureSet {
        match session.draft() {
            Some(draft) => draft
                .doc()
                .measures()
                .expect("a draft resolves against a body its document holds"),
            None => &self.loose,
        }
    }

    /// The body shaped while no product is open, which is written here and
    /// nowhere else.
    pub fn loose(&self) -> &MeasureSet {
        &self.loose
    }

    /// The shape the body is generated from. A body that never stored one
    /// stands at the adult default, and reading it writes nothing.
    pub fn shape(&self, session: &Session) -> BodyShape {
        self.body(session).phenotype.unwrap_or_default()
    }

    /// Where the body the tab shapes is kept.
    pub fn kept(session: &Session) -> Kept {
        if session.draft().is_some() {
            Kept::InProduct
        } else {
            Kept::Nowhere
        }
    }

    /// Whether another body of the product already carries `name`.
    ///
    /// The loose body is replaced rather than joined, so no name is taken
    /// while no product is open.
    pub fn name_taken(session: &Session, name: &str) -> bool {
        session
            .draft()
            .is_some_and(|draft| draft.doc().mannequin_named(name.trim()).is_some())
    }

    /// The first name counting up from `base` that no body of the product
    /// carries: a second new body is not refused for the default name.
    pub fn free_name(session: &Session, base: &str) -> String {
        let mut name = base.to_owned();
        let mut count = 1;
        while Self::name_taken(session, &name) {
            count += 1;
            name = format!("{base} {count}");
        }
        name
    }

    /// What the session refused, for as long as the document has not moved
    /// on from it.
    pub fn refused(&self, session: &Session) -> Option<&str> {
        self.refusal
            .as_ref()
            .filter(|(_, at)| *at == session.revision())
            .map(|(why, _)| why.as_str())
    }

    /// Whether a control is in hand.
    pub fn held(&self) -> bool {
        self.held.is_some()
    }

    /// Whether the body has to be solved again before it is painted.
    ///
    /// The body is compared, not the revision: whoever changed it — this tab,
    /// an undo in Patronaje, another body chosen to resolve with, another
    /// product opened at the same revision number — it is seen, and an undo
    /// back onto the tape the mesh was solved from costs nothing. Nothing is
    /// due while a control is in hand, since a drag asks for sixty frames a
    /// second and a solve costs milliseconds.
    pub fn due(&self, session: &Session) -> bool {
        let Some(built) = &self.built else {
            return true;
        };
        let set = self.body(session);
        !self.held() && (built.values != set.values || built.shape != self.shape(session))
    }

    /// Solves the levers against the body and lofts the mesh they give.
    ///
    /// The mesh comes from the solved phenotype and levers, not from the raw
    /// shape: the solve closes `estatura` through Anny's `height` input, and
    /// every other row it can through that row's own lever.
    pub fn rebuild(&mut self, session: &Session) -> BodyMesh {
        let set = self.body(session);
        let shape = self.shape(session);
        let solved = body::solve_anny(set, &body::phenotype_of(&shape));
        let mesh = body::body_mesh(&solved.phenotype, &solved.levers);
        let values = set.values.clone();
        self.built = Some(Basis { values, shape });
        self.stature_cm = Some(body::stature_cm(&mesh));
        self.solved = Some(solved);
        self.resolved = true;
        mesh
    }

    /// Whether the tape on the view is stale: the body was solved again, or
    /// another row is the one in hand. Neither, and nothing is laid, so a
    /// frame that changed nothing uploads nothing.
    pub fn tape_due(&self) -> bool {
        self.resolved || self.taped != self.highlight
    }

    /// The tape of the row in hand, laid on `mesh`, remembered as laid.
    pub fn lay(&mut self, mesh: &BodyMesh) -> Option<Tape> {
        self.resolved = false;
        self.taped.clone_from(&self.highlight);
        self.highlight
            .as_deref()
            .and_then(|name| body::tape(name, mesh))
    }
}

#[cfg(test)]
mod tests;
