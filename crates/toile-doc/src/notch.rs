use serde::{Deserialize, Serialize};

use crate::{EdgeAnchor, NotchKey};

/// A mark on a contour, and the mark it answers to on the other side.
///
/// A notch inside a seam is born with its twin in the same command, so an
/// orphan is impossible rather than merely discouraged. A lone notch is still
/// a notch: a centre front, or the mouth of a pocket.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Notch {
    /// Where on the contour the mark sits.
    pub at: EdgeAnchor,
    /// The notch it is sewn to, when it is sewn to one.
    #[serde(default)]
    pub mate: Option<NotchKey>,
    /// How many cuts the mark is.
    pub count: NotchCount,
}

/// How many cuts a notch is, which is how a sewer tells them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotchCount {
    /// One cut.
    Single,
    /// Two cuts.
    Double,
    /// Three cuts.
    Triple,
}

impl Notch {
    /// A single notch answering to nothing.
    pub fn lone(at: EdgeAnchor) -> Notch {
        Notch {
            at,
            mate: None,
            count: NotchCount::Single,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PieceKey, PointKey};

    #[test]
    fn a_lone_notch_answers_to_nothing() {
        let anchor = EdgeAnchor::at_node(PieceKey::new(0, 0), PointKey::new(1, 0));
        let notch = Notch::lone(anchor);
        assert_eq!(notch.mate, None);
        assert_eq!(notch.count, NotchCount::Single);
        assert_eq!(notch.at, anchor);
    }
}
