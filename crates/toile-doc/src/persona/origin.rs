use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::DocError;

/// Where a body in a document was copied from: enough to offer a refresh, and
/// nothing beyond the name the user already typed.
///
/// Nothing resolves through it. It is only read to ask the library whether
/// the person has changed since, and a library without the file answers no.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Origin {
    /// The stem of the library file, such as `ana`: the identity of the link,
    /// where the person's name is free text.
    pub persona: String,
    /// The day of the session copied, as `YYYY-MM-DD`.
    pub taken: String,
    /// The fingerprint of what was copied, as `Snapshot::fingerprint` writes
    /// it.
    pub fnv: String,
}

impl Origin {
    /// Whether `stem` could name a library file.
    ///
    /// Lowercase ASCII letters, digits, `_` and `-`, at least one of them:
    /// nothing that climbs out of the library, and nothing a case-folding file
    /// system would take for another person's file.
    pub fn is_stem(stem: &str) -> bool {
        !stem.is_empty()
            && stem
                .bytes()
                .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-'))
    }

    /// Refuses a link Toile could not have written.
    pub(crate) fn check(&self) -> Result<(), DocError> {
        if !Origin::is_stem(&self.persona) {
            return Err(DocError::NotAStem(self.persona.clone()));
        }
        day(&self.taken)?;
        let hex = self.fnv.len() == 16
            && self
                .fnv
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'));
        if !hex {
            return Err(DocError::NotAFingerprint(self.fnv.clone()));
        }
        Ok(())
    }
}

/// Refuses a day not written as `YYYY-MM-DD`.
///
/// Its shape and its ranges, not the calendar: the text orders sessions and is
/// shown to the user, and nothing counts days with it.
pub(super) fn day(text: &str) -> Result<(), DocError> {
    let number = |range: Range<usize>| {
        text.get(range)
            .filter(|part| part.bytes().all(|byte| byte.is_ascii_digit()))
            .and_then(|part| part.parse::<u32>().ok())
    };
    let dashed = text.len() == 10 && text.as_bytes()[4] == b'-' && text.as_bytes()[7] == b'-';
    match (dashed, number(0..4), number(5..7), number(8..10)) {
        (true, Some(_), Some(1..=12), Some(1..=31)) => Ok(()),
        _ => Err(DocError::NotADay(text.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stem_is_what_a_slug_leaves_and_nothing_that_leaves_the_library() {
        for stem in ["ana", "ana-maria", "talla_42", "ana-2"] {
            assert!(Origin::is_stem(stem), "{stem}");
        }
        for stem in [
            "",
            "Ana",
            "../ana",
            "ana.toile-persona",
            "ana maria",
            "ana/2",
            "núñez",
        ] {
            assert!(!Origin::is_stem(stem), "{stem}");
        }
    }

    #[test]
    fn a_day_is_written_year_month_day() {
        for good in ["2026-09-15", "1999-12-31", "2026-02-31"] {
            assert_eq!(day(good), Ok(()), "{good}");
        }
        for bad in [
            "",
            "2026-9-15",
            "2026-13-01",
            "2026-00-10",
            "2026-09-32",
            "15/09/2026",
            "2026-09-1x",
            "+026-09-15",
            "2026-09-15T10:00",
        ] {
            assert_eq!(day(bad), Err(DocError::NotADay(bad.to_owned())), "{bad}");
        }
    }
}
