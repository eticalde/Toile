/// The sheets around one sheet, where a sheet was printed at all.
///
/// A neighbour that was never printed is not a neighbour: there is nothing to
/// lap onto, so nothing on the sheet says to cut that edge and no mark is put
/// on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Joins {
    pub(super) above: Option<usize>,
    pub(super) below: Option<usize>,
    pub(super) left: Option<usize>,
    pub(super) right: Option<usize>,
}

/// One sheet's place in a piece: which cell of the grid it carries, and what it
/// is taped to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Place {
    /// Which sheet of the piece this is, counting only the sheets that are
    /// printed, so that it is also the page a print dialogue asks for.
    pub(super) number: usize,
    /// How many sheets the piece takes.
    pub(super) total: usize,
    /// Which column and row of the grid this sheet carries.
    pub(super) cell: [usize; 2],
    /// How many columns and rows the grid has, blank cells and all, because
    /// what a person wants to read is where this sheet sits in the whole shape.
    pub(super) counts: [usize; 2],
    /// The sheets this one is taped to.
    pub(super) joins: Joins,
}
