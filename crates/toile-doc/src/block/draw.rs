use crate::{Binding, Command, Doc, PieceKey, Point, SegmentEdit};

/// One tract a block bends: the node it leaves, the two handles that bend it,
/// and how finely it is flattened.
pub(super) struct Bend {
    /// The name of the node the tract leaves.
    pub from: &'static str,
    /// The handle leaving that node: its name, then its x and its y.
    pub out: (&'static str, &'static str, &'static str),
    /// The handle entering the next node, written the same way.
    pub into: (&'static str, &'static str, &'static str),
    /// How many points the tract contributes to the flattened contour.
    pub samples: u16,
}

/// Bends one tract of a piece, the way the curve tool bends one.
///
/// The two commands are the ones a gesture emits, in the order it emits them:
/// the count first, because a tract may not take handles until it is sampled
/// finely enough to show them. The handles land in the document under the
/// same rules a person's curve lands under, so the block cannot describe a
/// shape the editor could not have drawn.
pub(super) fn bend(doc: &mut Doc, piece: PieceKey, curve: &Bend) {
    let node = doc
        .shows_label(piece, curve.from)
        .expect("the block bends a node it has just named itself");
    Command::SetSamples {
        piece,
        node,
        to: curve.samples,
    }
    .apply(doc)
    .expect("the contour runs through the node the bend names");
    Command::SetSegment {
        piece,
        node,
        to: SegmentEdit::cubic(handle(curve.out), handle(curve.into)),
    }
    .apply(doc)
    .expect("the tract is sampled for a curve a count ago");
}

/// One handle of a curve as a point of the document, name and all.
fn handle((label, x, y): (&str, &str, &str)) -> Point {
    Point::at(binding(x), binding(y)).named(label)
}

/// The binding one of the block's own sources spells.
pub(super) fn binding(source: &str) -> Binding {
    Binding::parse(source).expect("the block's own sources are written beside this file and parse")
}
