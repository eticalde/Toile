use serde::{Deserialize, Serialize};

use super::canonical_bytes;
use crate::{Persona, PersonaError, Snapshot};

/// The format version of a library file.
///
/// A number of its own, apart from a pattern's: a person and a pattern are
/// extended on their own schedules, and neither stamp says anything about the
/// other file.
pub const VERSION: u32 = 1;

/// The extension a library file carries after its stem.
///
/// Part of the format rather than of the app, so that every program filing a
/// person writes the name the app will list.
pub const EXTENSION: &str = "toile-persona";

/// A library file: the version, and the person beside it.
#[derive(Serialize)]
struct Written<'a> {
    toile_persona: u32,
    name: &'a str,
    notes: &'a str,
    taken: &'a [Snapshot],
}

/// The person a library file carries.
///
/// The version is read first and on its own, for the reason a pattern's is.
#[derive(Deserialize)]
struct Loaded {
    name: String,
    #[serde(default)]
    notes: String,
    taken: Vec<Snapshot>,
}

impl Persona {
    /// The person as canonical JSON, ending in a newline, through the writer
    /// a pattern file is written with.
    ///
    /// One person has exactly one text, so a corrected measurement is one line
    /// of a diff and deleting the file deletes the person whole.
    ///
    /// # Errors
    /// What `from_json` would refuse the text for: no session, sessions out of
    /// order, a day not written `YYYY-MM-DD`, or a number JSON cannot spell.
    /// A save that went ahead would lose the person it was saving.
    ///
    /// # Panics
    /// Only if a JSON writer writes something that is not UTF-8, which is an
    /// invariant of the writer.
    pub fn to_canonical_json(&self) -> Result<String, PersonaError> {
        self.check()?;
        let mut out = canonical_bytes(&Written {
            toile_persona: VERSION,
            name: &self.name,
            notes: &self.notes,
            taken: &self.taken,
        });
        out.push(b'\n');
        Ok(String::from_utf8(out).expect("a JSON writer writes UTF-8"))
    }

    /// The person a library file holds.
    ///
    /// # Errors
    /// `PersonaError`, naming what is wrong with the file: text that is not
    /// JSON, JSON that stops early, a missing or unknown version, a shape that
    /// is not a person's, or a person the writer would refuse to write.
    pub fn from_json(text: &str) -> Result<Persona, PersonaError> {
        let found = version(text)?;
        if found != u64::from(VERSION) {
            return Err(PersonaError::UnknownVersion {
                found,
                newest: VERSION,
            });
        }
        let loaded: Loaded =
            serde_json::from_str(text).map_err(|error| PersonaError::while_reading(&error))?;
        let persona = Persona {
            name: loaded.name,
            notes: loaded.notes,
            taken: loaded.taken,
        };
        persona.check()?;
        Ok(persona)
    }
}

/// The format version a library file declares.
fn version(text: &str) -> Result<u64, PersonaError> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| PersonaError::while_reading(&error))?;
    value
        .get("toile_persona")
        .and_then(serde_json::Value::as_u64)
        .ok_or(PersonaError::NoHeader)
}
