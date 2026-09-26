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

/// The format version of a document in which a piece carries an internal line.
///
/// Under the elastic's rule it takes the next number. A build that reads
/// version 5 predates the internal line, so it would open the product, drop
/// every fold, topstitch, pocket mouth, buttonhole and mark, and save back a
/// pattern that cannot be cut from paper — with nothing on screen to say a
/// line was ever there.
pub const VERSION_INTERNAL: u32 = 6;

/// The format version of a document in which a piece is drawn against an axis.
///
/// Under the internal line's rule it takes the next number. A build that reads
/// version 6 predates the axis, so it would open the product, drop the fold,
/// and cut, mesh, drape and export half a waistband as if it were the whole
/// one — a piece exactly half the width it says it is, with nothing on screen
/// to say a fold was ever there.
pub const VERSION_FOLDED: u32 = 7;

/// The format version of a document in which a stretch of contour is hung from
/// a station of the body.
///
/// Under the axis's rule it takes the next number. A build that reads version 7
/// predates the hang, so it would open the product, drop the one thing that
/// says where on the body the garment belongs, and drape the same file into a
/// garment that slides off — the elastic's own case, one step further on.
pub const VERSION_HUNG: u32 = 8;

/// The format version of a document in which a contour carries a dart.
///
/// Under the hang's rule it takes the next number, and it is the axis's case
/// rather than the elastic's: a build that reads version 8 carries the record
/// through a save untouched, because the wedge is ordinary contour nodes and
/// the seam that closes it an ordinary seam. What it cannot do is read those
/// three nodes as a wedge — so it lets the person move one leg, take a node
/// out from between the legs or unpick the seam, and saves back a dart whose
/// record no longer describes the contour it names, or none the reader will
/// open at all.
pub const VERSION_DARTED: u32 = 9;

impl Doc {
    /// The format version this document is written in.
    ///
    /// A pure function of what the document carries, never of the file it was
    /// read from, so one document still has exactly one text: the highest
    /// number any body, piece, elastic, internal line, axis, hang or dart in
    /// it needs.
    pub fn format_version(&self) -> u32 {
        let bodies = self.mannequins.iter().map(|(_, set)| set.format_version());
        let pieces = self.pieces.iter().map(|(_, piece)| piece.format_version());
        // Neither an elastic nor an internal line has an older spelling to fall
        // back to, so one of either is enough to make the whole document ask
        // for its version.
        let elastic = (!self.elastics.is_empty()).then_some(VERSION_ELASTIC);
        let lines = (!self.lines.is_empty()).then_some(VERSION_INTERNAL);
        let folded = (!self.symmetries.is_empty()).then_some(VERSION_FOLDED);
        let hung = (!self.hangs.is_empty()).then_some(VERSION_HUNG);
        let darted = (!self.darts.is_empty()).then_some(VERSION_DARTED);
        bodies
            .chain(pieces)
            .chain(elastic)
            .chain(lines)
            .chain(folded)
            .chain(hung)
            .chain(darted)
            .max()
            .unwrap_or(VERSION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MeasureSet, block};

    /// Every number is the one after the last, and each is a number a reader
    /// that predates it refuses rather than a field it drops in silence.
    #[test]
    fn each_version_is_the_one_after_the_field_before_it() {
        let numbers = [
            VERSION,
            VERSION_EXTENDED,
            VERSION_LINKED,
            VERSION_PLACED,
            VERSION_ELASTIC,
            VERSION_INTERNAL,
            VERSION_FOLDED,
            VERSION_HUNG,
            VERSION_DARTED,
        ];
        for (at, number) in numbers.iter().enumerate() {
            assert_eq!(*number, at as u32 + 1, "version {at}");
        }
    }

    /// A document that carries none of the newer entities asks for the oldest
    /// version there is, which is what keeps every file on disk as it was.
    #[test]
    fn a_document_carrying_nothing_newer_asks_for_the_first_version() {
        assert_eq!(Doc::new(MeasureSet::default()).format_version(), VERSION);
        assert_eq!(block::trousers().format_version(), VERSION);
    }
}
