use thiserror::Error;

/// Where a formula's source text stops being a formula.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{kind} at byte {at}")]
pub struct SyntaxError {
    /// Byte offset into the source where the fault was found.
    pub at: usize,
    /// What was wrong there.
    pub kind: SyntaxKind,
}

/// The faults a formula's source text can have.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SyntaxKind {
    /// The source holds no expression at all.
    #[error("the formula is empty")]
    Empty,
    /// A character the language has no token for.
    #[error("unexpected character `{0}`")]
    UnexpectedChar(char),
    /// Digits that are not a finite decimal number.
    #[error("not a finite decimal number")]
    MalformedNumber,
    /// A call to a name that is not one of the four functions.
    #[error("unknown function `{0}`")]
    UnknownFunction(String),
    /// An operand is missing.
    #[error("expected an expression")]
    ExpectedExpression,
    /// A function name that is not followed by its argument list.
    #[error("expected `(`")]
    ExpectedOpeningParen,
    /// A group or an argument list that never closes.
    #[error("expected `)`")]
    ExpectedClosingParen,
    /// A `?` whose condition is not a comparison.
    #[error("expected a comparison before `?`")]
    ExpectedComparison,
    /// A comparison that decides nothing.
    #[error("expected `?` after a comparison")]
    ExpectedQuestionMark,
    /// A conditional with no alternative.
    #[error("expected `:`")]
    ExpectedColon,
    /// A call with the wrong number of arguments.
    #[error("expected {expected} argument(s), found {found}")]
    WrongArgumentCount {
        /// How many the function takes.
        expected: usize,
        /// How many were written.
        found: usize,
    },
    /// Source left over once the expression ended.
    #[error("unexpected input after the expression")]
    TrailingInput,
    /// Nesting deep enough to threaten the stack.
    #[error("the expression nests too deeply")]
    TooDeep,
    /// More tokens than the recursive walks over the tree can carry.
    #[error("the formula is too long")]
    TooLong,
}

impl SyntaxError {
    /// A fault of `kind`, found at byte offset `at`.
    pub fn new(at: usize, kind: SyntaxKind) -> SyntaxError {
        SyntaxError { at, kind }
    }
}
