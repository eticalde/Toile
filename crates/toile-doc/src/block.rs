mod back;
mod draw;
mod front;

pub use back::BACK;
pub use front::{FRONT, trouser_front};

use crate::Doc;

/// The trouser block Toile ships: the front, the back, and the two seams
/// that close a leg.
///
/// This is the document the asset file is generated from. The front alone
/// remains reachable for the tests and goldens that watch one piece.
pub fn trousers() -> Doc {
    let mut doc = front::trouser_front();
    back::trouser_back(&mut doc);
    doc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SeamKind, Winding};

    #[test]
    fn the_block_puts_two_pieces_on_the_table_and_sews_them_twice() {
        let doc = trousers();
        let front = doc.piece_named(FRONT).expect("the front is drawn");
        let back = doc.piece_named(BACK).expect("the back is drawn");
        assert_eq!(doc.pieces.len(), 2);
        assert_eq!(doc.seams.len(), 2);
        for (_, seam) in doc.seams.iter() {
            assert_eq!(seam.kind, SeamKind::Plain);
            assert_eq!(seam.a.piece(), Some(front));
            assert_eq!(seam.b.piece(), Some(back));
        }
    }

    #[test]
    fn the_back_is_drawn_the_same_way_round_as_the_front() {
        let doc = trousers();
        let back = doc.piece_named(BACK).expect("the back is drawn");
        let held = doc.pieces.get(back).expect("the key is live");
        assert_eq!(held.winding, Winding::Cw);
        assert_eq!(held.contour.len(), 9);
    }
}
