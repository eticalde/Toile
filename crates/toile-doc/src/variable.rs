use serde::{Deserialize, Serialize};

use crate::Binding;

/// A quantity the pattern names once and reads wherever it needs it.
///
/// Its identity is its key, not its name and not its position: renaming a
/// variable rewrites the formulas that read it, and removing one does not
/// renumber the rest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    /// The name formulas read it by.
    pub name: String,
    /// What it is bound to, in centimetres or as a plain ratio.
    pub value: Binding,
}

impl Variable {
    /// A variable bound to what `value` makes of it.
    pub fn new(name: &str, value: impl Into<Binding>) -> Variable {
        Variable {
            name: name.to_owned(),
            value: value.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_variable_carries_a_binding_like_a_coordinate_does() {
        let ease = Variable::new("holgura_cadera", 1.0);
        assert_eq!(ease.name, "holgura_cadera");
        assert_eq!(ease.value, Binding::Literal(1.0));
    }

    #[test]
    fn a_variable_may_be_written_as_a_formula() {
        let source = Binding::parse("cadera / 16").expect("the source parses");
        let rise = Variable::new("extension_tiro", source);
        assert_eq!(rise.value.source(), "cadera / 16");
        assert_eq!(
            rise.value.names().into_iter().collect::<Vec<_>>(),
            ["cadera"]
        );
    }
}
