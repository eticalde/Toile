use crate::PieceKey;

/// A point of a piece held to a place in space.
///
/// The rest position is the one in the flat piece, which is what the transfer
/// between two meshes knows how to locate: the 2D pattern stays the only
/// source of geometric truth, and the drag in the viewer is stored against it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pin {
    /// The piece the pinned material belongs to.
    pub piece: PieceKey,
    /// Where on the flat piece it is, in centimetres.
    pub rest: [f64; 2],
    /// Where it is held, in metres of world space.
    pub to: [f32; 3],
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "a pin stores the position it was given")]

    use super::*;

    #[test]
    fn a_pin_names_the_piece_it_holds() {
        let pin = Pin {
            piece: PieceKey::new(0, 0),
            rest: [10.0, 20.0],
            to: [0.0, 1.0, 0.0],
        };
        assert_eq!(pin.piece, PieceKey::new(0, 0));
        assert_eq!(pin.rest, [10.0, 20.0]);
    }
}
