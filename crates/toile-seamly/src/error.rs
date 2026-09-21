use std::fmt;

use thiserror::Error;

use crate::FormulaError;

/// Where in the file something went wrong: the element, as a reader finds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// The 1-based line the element starts on.
    pub line: u32,
    /// The element as the file opens it, reduced to its `type`, `id` and
    /// `name`, e.g. `<point type="cutSpline" id="233" name="ff_hook">`.
    pub element: String,
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, {}", self.line, self.element)
    }
}

/// What can go wrong reading or evaluating a Seamly file.
///
/// Every variant but the first names the element at fault, so an unsupported
/// construction reads as a sentence about the pattern, not about this crate.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum Error {
    /// The text is not well-formed XML.
    #[error("not well-formed XML: {0}")]
    Xml(String),
    /// A required attribute is missing or unreadable, or a reference is
    /// dangling.
    #[error("{at}: {what}")]
    Malformed {
        /// The element at fault.
        at: Place,
        /// What is wrong with it.
        what: String,
    },
    /// Something the file carries that this crate does not model.
    #[error("{at}: {what} is not supported")]
    Unsupported {
        /// The element at fault.
        at: Place,
        /// The construction, attribute or element it does not model.
        what: String,
    },
    /// A formula that does not parse or does not evaluate.
    #[error("{at}: `{attribute}=\"{formula}\"`: {source}")]
    Formula {
        /// The element whose attribute holds the formula.
        at: Place,
        /// The attribute, e.g. `length`.
        attribute: &'static str,
        /// The formula as the file writes it.
        formula: String,
        /// Why it failed.
        source: FormulaError,
    },
    /// A construction with no answer: parallel lines, a line of length zero,
    /// a cut past the end of its curve.
    #[error("{at}: {what}")]
    Geometry {
        /// The element at fault.
        at: Place,
        /// Why it has no answer.
        what: String,
    },
    /// A pattern that reads and evaluates, but that a Toile product cannot
    /// say: a name no formula could spell, two names that collide, a piece
    /// whose outline does not close.
    #[error("{0}")]
    Product(String),
}
