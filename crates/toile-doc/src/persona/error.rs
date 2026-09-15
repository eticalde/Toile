use serde_json::error::Category;
use thiserror::Error;

use crate::DocError;

/// What can be wrong with a file that claims to be a person, or with a person
/// on the way to a file or a document.
///
/// A library file is as likely to be edited by hand as a pattern, so every
/// variant names what is wrong with the person rather than what the reader
/// was doing when it found out.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PersonaError {
    /// The text is not JSON at all.
    #[error("the file is not JSON: {0}")]
    NotJson(String),
    /// The text is JSON that stops early.
    #[error("the file ends before the person does: {0}")]
    Truncated(String),
    /// JSON with no version header, so nothing says it is a person.
    #[error("the file carries no `toile_persona` version number, so it is not a person")]
    NoHeader,
    /// A version this build does not know how to read.
    #[error(
        "the person is written in format version {found}; this build reads up to version {newest}"
    )]
    UnknownVersion {
        /// The version the file declares.
        found: u64,
        /// The newest version this build understands.
        newest: u32,
    },
    /// JSON of the right version that is not shaped like a person.
    #[error("the person is not shaped like one: {0}")]
    Malformed(String),
    /// A person with no session with the tape, whom no document could copy.
    #[error("the person has no session with the tape, and a person holds at least one")]
    NothingTaken,
    /// Sessions that do not run oldest first.
    #[error(
        "the session of {listed} is listed after the one of {after}; sessions run oldest first"
    )]
    OutOfOrder {
        /// The day of the session listed second.
        listed: String,
        /// The later day of the session listed before it.
        after: String,
    },
    /// A value inside the person that a file could not hold.
    #[error(transparent)]
    Invalid(#[from] DocError),
}

impl PersonaError {
    /// What a reader makes of a failure to read.
    pub(crate) fn while_reading(error: &serde_json::Error) -> PersonaError {
        let told = error.to_string();
        match error.classify() {
            Category::Eof => PersonaError::Truncated(told),
            Category::Syntax => PersonaError::NotJson(told),
            Category::Data | Category::Io => PersonaError::Malformed(told),
        }
    }
}
