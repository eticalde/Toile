mod binding;
mod check;
mod error;
mod id;
mod number;
mod persona;
mod store;
mod writer;

pub use error::FormatError;
pub use persona::{EXTENSION as PERSONA_EXTENSION, VERSION as PERSONA_VERSION};
use serde::{Deserialize, Serialize};
use writer::Canonical;

use crate::Doc;

/// The format version of a document that carries nothing newer.
///
/// A document keeps this stamp for as long as it can, so that a pattern an
/// older Toile wrote is written back as the very bytes it was read from.
pub const VERSION: u32 = 1;

/// The format version of a document in which some body carries a phenotype.
///
/// A version-1 reader would drop it without a word and generate a body its
/// owner never shaped. Under this stamp it refuses the file for its version.
pub const VERSION_EXTENDED: u32 = 2;

/// The format version of a document in which some body carries a link to the
/// library.
///
/// It cannot share the phenotype's stamp: builds that read version 2 predate
/// the link, so they would accept such a file, drop the link, and never again
/// offer to update a copy the library has moved on from. Each field a reader
/// could lose takes the next number, so the reader that would lose it refuses.
pub const VERSION_LINKED: u32 = 3;

/// The format version of a document in which some piece carries a placement
/// on the product overview.
///
/// Under the link's rule it takes the next number: builds that read version 3
/// predate it, and would open an arranged product, drop every placement, and
/// save it back with its pieces wherever the overview first laid them.
pub const VERSION_PLACED: u32 = 4;

/// The format version of a document that holds an elastic.
///
/// Under the placement's rule it takes the next number, and here the rule bites
/// hardest: a build that reads version 4 predates the elastic, so it would open
/// the product, drop what holds the garment on the body, and drape the same
/// file into a different garment.
pub const VERSION_ELASTIC: u32 = 5;

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
    /// The format version this document is written in.
    ///
    /// A pure function of what the document carries, never of the file it was
    /// read from, so one document still has exactly one text: the highest
    /// number any body, piece or elastic in it needs.
    pub fn format_version(&self) -> u32 {
        let bodies = self.mannequins.iter().map(|(_, set)| set.format_version());
        let pieces = self.pieces.iter().map(|(_, piece)| piece.format_version());
        // An elastic has no older spelling to fall back to, so one of them is
        // enough to make the whole document ask for its version.
        let elastic = (!self.elastics.is_empty()).then_some(VERSION_ELASTIC);
        bodies.chain(pieces).chain(elastic).max().unwrap_or(VERSION)
    }

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
        let mut out = canonical_bytes(&Written {
            toile: self.format_version(),
            doc: self,
        });
        out.push(b'\n');
        String::from_utf8(out).expect("a JSON writer writes UTF-8")
    }

    /// The document a file holds.
    ///
    /// # Errors
    /// `FormatError`, naming what is wrong with the file: text that is not
    /// JSON, JSON that stops early, a missing or unknown version, a shape that
    /// is not a pattern's, a key that leads nowhere, a tract asking to be
    /// flattened at a count no tract can carry, a link to the library Toile
    /// could not have written, or an elastic holding a stretch to numbers no
    /// elastic holds.
    pub fn from_json(text: &str) -> Result<Doc, FormatError> {
        let found = version(text)?;
        if !(u64::from(VERSION)..=u64::from(VERSION_ELASTIC)).contains(&found) {
            return Err(FormatError::UnknownVersion {
                found,
                newest: VERSION_ELASTIC,
            });
        }
        let loaded: Loaded =
            serde_json::from_str(text).map_err(|error| FormatError::while_reading(&error))?;
        check::references(&loaded.doc)?;
        check::samplings(&loaded.doc)?;
        check::origins(&loaded.doc)?;
        check::elastics(&loaded.doc)?;
        Ok(loaded.doc)
    }
}

/// `value` as the canonical writer spells it, with no newline after it.
///
/// A pattern, a library file and a fingerprint are all written by this one
/// writer, so a number means the same bytes in each of them.
pub(crate) fn canonical_bytes<T: Serialize + ?Sized>(value: &T) -> Vec<u8> {
    let mut out = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(&mut out, Canonical::new());
    value
        .serialize(&mut serializer)
        .expect("every map this crate writes is keyed by strings");
    out
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
    use crate::{Command, Doc, EdgeRange, Elastic, Identity, MeasureSet, block};

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

    /// A hand-edited elastic is refused the way a hand-edited sampling is.
    /// Not every number a file can spell is one an elastic holds, and the
    /// difference reaches the solver as a rest length and as a compliance. A
    /// strength of `1e-300` is the case "above zero" let through: positive,
    /// finite, and a waistline that runs at no stiffness at all.
    #[test]
    fn an_elastic_the_file_carries_is_checked_before_the_pattern_opens() {
        let mut doc = block::trouser_front();
        let front = doc.piece_named(block::FRONT).expect("the block draws one");
        let at = {
            let named = |label| doc.shows_label(front, label).expect("the block names it");
            EdgeRange::between(front, named("cintura_cf"), named("cadera_lat"))
        };
        Command::AddElastic {
            identity: Identity::New,
            elastic: Elastic::new(at, 0.85, 10.0),
        }
        .apply(&mut doc)
        .expect("both ends are nodes of the front");
        let written = doc.to_canonical_json();
        assert_eq!(Doc::from_json(&written), Ok(doc));
        for hand in [
            "\"ratio\": -5",
            "\"ratio\": 1e9",
            "\"strength\": 0",
            "\"strength\": 1e-300",
        ] {
            let (key, _) = hand.split_once(':').expect("every case names its key");
            let was = if key == "\"ratio\"" {
                "\"ratio\": 0.85"
            } else {
                "\"strength\": 10"
            };
            let edited = written.replace(was, hand);
            assert_ne!(edited, written, "{hand} left the file as it was");
            let error = Doc::from_json(&edited).expect_err(hand);
            assert!(matches!(error, FormatError::Elastic(_)), "{hand}: {error}");
        }
    }
}
