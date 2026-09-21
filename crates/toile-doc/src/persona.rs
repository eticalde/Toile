mod error;
pub(crate) mod fingerprint;
mod origin;
mod slug;

use std::collections::BTreeMap;

pub use error::PersonaError;
pub use origin::Origin;
use origin::day;
use serde::{Deserialize, Serialize};

use crate::measure::finite;
use crate::{BodyShape, Command, DocError, MannequinKey, MeasureSet};

/// A person in the user's library: a named body, measured over time.
///
/// The library file is the person at rest. A document never points at the
/// library; it carries a copy, and that keeps a pattern file whole on its own.
#[derive(Debug, Clone, PartialEq)]
pub struct Persona {
    /// The name the library shows. User data, like the measurement names.
    pub name: String,
    /// Free text that never leaves the library file.
    ///
    /// A body and an edit are built from a `Snapshot`, and a snapshot has no
    /// field the notes could travel in.
    pub notes: String,
    /// The sessions with the tape, oldest first, and never none in a file.
    ///
    /// History grows and is never overwritten: a correction is a new session.
    pub taken: Vec<Snapshot>,
}

/// One session with the tape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    /// The day, as `YYYY-MM-DD`. A string, so that no clock enters the format.
    pub date: String,
    /// The measurements, by name, in centimetres.
    pub values: BTreeMap<String, f64>,
    /// What the body was generated from besides the tape, once someone set it.
    ///
    /// Without it, a body taken from the library and saved back to it would
    /// come back with a shape its owner never gave it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phenotype: Option<BodyShape>,
}

impl Persona {
    /// The session a document copies: the newest one.
    pub fn current(&self) -> Option<&Snapshot> {
        self.taken.last()
    }

    /// The current session as a body for a document, stamped with where it
    /// came from. `stem` is the library file's stem, the identity of the link.
    ///
    /// # Errors
    /// `PersonaError::NothingTaken` for a person with no session, and
    /// `PersonaError::Invalid` for a stem that could name no library file or a
    /// session a document could not hold.
    pub fn to_mannequin(&self, stem: &str) -> Result<MeasureSet, PersonaError> {
        let (current, origin) = self.link(stem)?;
        Ok(MeasureSet {
            name: self.name.clone(),
            values: current.values.clone(),
            phenotype: current.phenotype,
            origin: Some(origin),
        })
    }

    /// The edit that brings a body in a document up to the current session.
    ///
    /// It answers the offer to update a copy the library has moved on from,
    /// and it relinks a body whose tape was just saved to the library as a new
    /// session, since the fingerprint its old link carries is not that one's.
    ///
    /// # Errors
    /// The same as `to_mannequin`.
    pub fn refresh(&self, stem: &str, mannequin: MannequinKey) -> Result<Command, PersonaError> {
        let (current, origin) = self.link(stem)?;
        Ok(Command::RefreshMannequin {
            mannequin,
            values: current.values.clone(),
            phenotype: current.phenotype,
            origin: Some(origin),
        })
    }

    /// Whether the current session is no longer what `origin` copied.
    ///
    /// Only the fingerprint is compared: a later session with the same tape
    /// and shape, a new name or new notes leave a copy as current as it was.
    pub fn differs_from(&self, origin: &Origin) -> bool {
        self.current()
            .is_some_and(|current| current.fingerprint() != origin.fnv)
    }

    /// The current session and the link a copy of it carries.
    fn link(&self, stem: &str) -> Result<(&Snapshot, Origin), PersonaError> {
        let current = self.current().ok_or(PersonaError::NothingTaken)?;
        current.check()?;
        let origin = Origin {
            persona: stem.to_owned(),
            taken: current.date.clone(),
            fnv: current.fingerprint(),
        };
        origin.check()?;
        Ok((current, origin))
    }

    /// Checks everything a library file promises about the person it holds.
    pub(crate) fn check(&self) -> Result<(), PersonaError> {
        if self.taken.is_empty() {
            return Err(PersonaError::NothingTaken);
        }
        for snapshot in &self.taken {
            snapshot.check()?;
        }
        // The newest session is the one a document copies, so a file that
        // lists them out of order would hand over an older one as current.
        // A day written YYYY-MM-DD orders as text the way it orders as a day.
        match self
            .taken
            .windows(2)
            .find(|pair| pair[1].date < pair[0].date)
        {
            Some(pair) => Err(PersonaError::OutOfOrder {
                listed: pair[1].date.clone(),
                after: pair[0].date.clone(),
            }),
            None => Ok(()),
        }
    }
}

impl Snapshot {
    /// The fingerprint a copy of this session is stamped with.
    ///
    /// FNV-1a 64, as sixteen lowercase hex digits, of the canonical JSON of
    /// the tape and the phenotype alone: `{"values": …, "phenotype": …}`,
    /// indented as a file is, with no key for a phenotype the session lacks.
    /// The text is written from the numbers and never read from a file, so a
    /// file's key order and spacing cannot reach it, and the name, the notes
    /// and the date are not in it. Every number is written the one shortest
    /// way that reads back as itself, so a value or a scale that moves by a
    /// single ulp moves the text.
    pub fn fingerprint(&self) -> String {
        fingerprint::of(&self.values, self.phenotype.as_ref())
    }

    /// Refuses a session a file could not hold.
    fn check(&self) -> Result<(), DocError> {
        day(&self.date)?;
        finite(&self.values)?;
        self.phenotype.as_ref().map_or(Ok(()), BodyShape::check)
    }
}
