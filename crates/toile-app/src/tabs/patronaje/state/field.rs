use toile_engine::draft::{Axis, PieceKey, PointKey, VariableKey};

/// A field of the inspector somebody is writing in.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Field {
    /// One coordinate of one node.
    Coordinate(PointKey, Axis),
    /// How finely the tract leaving one node is flattened.
    Samples(PointKey),
    /// How far along the tract leaving one node its notch is cut.
    ///
    /// Named for the tract and not for the mark, the way the row is: the panel
    /// edits the first notch the chosen tract carries.
    Along(PointKey),
    /// One of the things a piece says about being cut out.
    Cut(PieceKey, Cut),
    /// One measurement of the body the pattern resolves against.
    Measure(String),
    /// One of the pattern's own quantities.
    Variable(VariableKey),
}

/// Which of the things a piece says about being cut out a row writes.
///
/// A label line is named by where it sits in the label, because that is all
/// the document gives a line to be known by: the lines are a sequence its
/// writer typed, not a set of named parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cut {
    /// How far outside the drawn line the cloth is cut.
    Allowance,
    /// How many of the piece the garment takes.
    Quantity,
    /// The letter the piece's label shows.
    Letter,
    /// One line of the label, by its place in it.
    Label(usize),
}

/// The text a field holds while it is being written, before it parses.
///
/// The buffer lives here and not in the document, which is what lets a field
/// paint the fault in what has been typed so far without the geometry ever
/// seeing it.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldEdit {
    /// The field the text belongs to.
    pub of: Field,
    /// What has been typed into it.
    pub buffer: String,
}
