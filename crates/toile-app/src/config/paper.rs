use serde::{Deserialize, Deserializer, Serialize};
use toile_engine::export;

/// The size of paper the studio lays a pattern out on.
///
/// Kept as a word and not as a size in millimetres: what a person chose is
/// «Carta», and the millimetres that word stands for belong to the engine that
/// lays the sheets out, which is where they can be checked against the standard
/// that states them.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Paper {
    /// The size the studio laid every print out on before the choice existed,
    /// so a preferences file that says nothing prints as it always did.
    #[default]
    A4,
    /// The size sold by the ream across most of the Americas.
    Carta,
}

impl Paper {
    /// The sheet the engine lays a pattern out on for this choice.
    pub fn sheet(self) -> export::Paper {
        match self {
            Paper::A4 => export::A4,
            Paper::Carta => export::CARTA,
        }
    }

    /// What the paper is called, in the studio and on the printed page.
    ///
    /// Asked of the sheet rather than spelled again here, so the name on the
    /// button is the name the legend of every sheet prints.
    pub fn name(self) -> &'static str {
        self.sheet().name
    }

    /// The next size in order.
    pub fn next(self) -> Paper {
        match self {
            Paper::A4 => Paper::Carta,
            Paper::Carta => Paper::A4,
        }
    }
}

/// The paper a preferences file names, or none where it names one this build
/// does not know.
///
/// Read forgivingly on purpose. A word from a later build is a preference to
/// forget, and read strictly it would fail the whole file and take the window
/// geometry and the recent patterns down with it — which is the same reason the
/// version is not bumped for a field that was added.
pub(super) fn known<'de, D>(deserializer: D) -> Result<Option<Paper>, D::Error>
where
    D: Deserializer<'de>,
{
    let word = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(word.and_then(|word| serde_json::from_value(word).ok()))
}
