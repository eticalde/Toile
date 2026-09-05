mod binding;
mod check;
mod error;
mod id;
mod number;
mod store;
mod writer;

pub use error::FormatError;
use serde::{Deserialize, Serialize};
use writer::Canonical;

use crate::Doc;

/// The version of the file format this build writes, and the only one it
/// reads.
pub const VERSION: u32 = 1;

/// A file: the version, and the pattern under it.
#[derive(Serialize)]
struct Written<'a> {
    toile: u32,
    doc: &'a Doc,
}

/// The pattern a file carries.
///
/// The version is read first and on its own, so that a file from a later Toile
/// is refused for its version rather than for the first field of it this build
/// happens not to understand.
#[derive(Deserialize)]
struct Loaded {
    doc: Doc,
}

impl Doc {
    /// The document as canonical JSON, ending in a newline.
    ///
    /// One document has exactly one text: every collection is written in key
    /// order, every key as `index.generation`, every formula as the source its
    /// author typed and every number in the shortest form that reads back as
    /// itself. That is what makes moving one point one line of a diff, and
    /// what lets a reader — a person or a language model — follow a pattern
    /// from its measurements to its contour.
    ///
    /// # Panics
    /// Only if a JSON writer refuses a value a document can hold, or writes
    /// something that is not UTF-8. Both are invariants of the writer, so
    /// neither is anything a caller can do.
    pub fn to_canonical_json(&self) -> String {
        let mut out = Vec::new();
        let mut serializer = serde_json::Serializer::with_formatter(&mut out, Canonical::new());
        Written {
            toile: VERSION,
            doc: self,
        }
        .serialize(&mut serializer)
        .expect("a document holds no value a JSON writer can refuse");
        out.push(b'\n');
        String::from_utf8(out).expect("a JSON writer writes UTF-8")
    }

    /// The document a file holds.
    ///
    /// # Errors
    /// `FormatError`, naming what is wrong with the file: text that is not
    /// JSON, JSON that stops early, a missing or unknown version, a shape that
    /// is not a pattern's, a key that leads nowhere, or a tract asking to be
    /// flattened at a count no tract can carry.
    pub fn from_json(text: &str) -> Result<Doc, FormatError> {
        let found = version(text)?;
        if found != u64::from(VERSION) {
            return Err(FormatError::UnknownVersion {
                found,
                supported: VERSION,
            });
        }
        let loaded: Loaded =
            serde_json::from_str(text).map_err(|error| FormatError::while_reading(&error))?;
        check::references(&loaded.doc)?;
        check::samplings(&loaded.doc)?;
        Ok(loaded.doc)
    }
}

/// The format version a file declares.
fn version(text: &str) -> Result<u64, FormatError> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| FormatError::while_reading(&error))?;
    value
        .get("toile")
        .and_then(serde_json::Value::as_u64)
        .ok_or(FormatError::NoHeader)
}

#[cfg(test)]
mod tests {
    use crate::json::FormatError;
    use crate::{Doc, MeasureSet, block};

    #[test]
    fn a_file_begins_with_the_format_and_its_version() {
        let written = block::trouser_front().to_canonical_json();
        assert!(
            written.starts_with("{\n  \"toile\": 1,\n  \"doc\": {\n"),
            "{written}"
        );
        assert!(written.ends_with("}\n"));
    }

    #[test]
    fn an_empty_document_is_a_file_like_any_other() {
        let doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
        let written = doc.to_canonical_json();
        assert_eq!(Doc::from_json(&written), Ok(doc));
    }

    #[test]
    fn a_file_with_no_version_is_not_taken_for_a_pattern() {
        assert_eq!(Doc::from_json("{}"), Err(FormatError::NoHeader));
        assert_eq!(
            Doc::from_json("{\"toile\": \"1\", \"doc\": {}}"),
            Err(FormatError::NoHeader)
        );
    }
}
