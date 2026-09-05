use serde_json::error::Category;
use thiserror::Error;

use crate::DocError;

/// What can be wrong with a file that claims to be a pattern.
///
/// Every variant names what is wrong with the file rather than what the reader
/// was doing when it found out: opening a file someone edited by hand is the
/// ordinary case, and a panic is never the answer to it.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FormatError {
    /// The text is not JSON at all.
    #[error("the file is not JSON: {0}")]
    NotJson(String),
    /// The text is JSON that stops early.
    #[error("the file ends before the pattern does: {0}")]
    Truncated(String),
    /// JSON with no version header, so nothing says it is a pattern.
    #[error("the file carries no `toile` version number, so it is not a pattern")]
    NoHeader,
    /// A version this build does not know how to read.
    #[error("the file is written in format version {found}; this build reads version {supported}")]
    UnknownVersion {
        /// The version the file declares.
        found: u64,
        /// The version this build understands.
        supported: u32,
    },
    /// JSON of the right version that is not shaped like a pattern.
    #[error("the pattern is not shaped like one: {0}")]
    Malformed(String),
    /// A reference to an entity the file does not carry.
    #[error("the pattern points at something the file does not carry: {0}")]
    Dangling(#[source] DocError),
    /// A tract asking to be flattened at a count no tract can carry.
    #[error("the pattern asks for a flattening no tract can carry: {0}")]
    Sampling(#[source] DocError),
}

impl FormatError {
    /// What a reader makes of a failure to read.
    ///
    /// The category is what separates a file that was cut off from one that
    /// was never a pattern, and the two call for different answers from
    /// whoever opened it.
    pub(super) fn while_reading(error: &serde_json::Error) -> FormatError {
        let told = error.to_string();
        match error.classify() {
            Category::Eof => FormatError::Truncated(told),
            Category::Syntax => FormatError::NotJson(told),
            Category::Data | Category::Io => FormatError::Malformed(told),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(text: &str) -> FormatError {
        let error = serde_json::from_str::<serde_json::Value>(text).expect_err("the text is bad");
        FormatError::while_reading(&error)
    }

    #[test]
    fn a_file_cut_off_mid_pattern_is_told_apart_from_one_that_is_not_json() {
        assert!(matches!(
            reading("{\"toile\": 1, \"doc\": {"),
            FormatError::Truncated(_)
        ));
        assert!(matches!(
            reading("not json at all"),
            FormatError::NotJson(_)
        ));
    }

    #[test]
    fn an_unknown_version_names_both_versions() {
        let error = FormatError::UnknownVersion {
            found: 7,
            supported: 1,
        };
        assert_eq!(
            error.to_string(),
            "the file is written in format version 7; this build reads version 1"
        );
    }
}
