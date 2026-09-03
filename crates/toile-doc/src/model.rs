/// A pattern piece: a closed control polygon, in metres.
#[allow(missing_docs, reason = "one field, named by the doc above it")]
#[derive(Debug, Clone, Default)]
pub struct Piece {
    pub contour: Vec<[f64; 2]>,
}

/// The document: every piece on the work table.
#[allow(missing_docs, reason = "one field, named by the doc above it")]
#[derive(Debug, Clone, Default)]
pub struct Doc {
    pub pieces: Vec<Piece>,
}

/// A reversible edit to the document.
#[allow(
    missing_docs,
    reason = "the variant names the edit and its fields name themselves"
)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    MovePoint {
        piece: usize,
        point: usize,
        to: [f64; 2],
    },
}

impl Command {
    /// Applies the edit and returns the command that undoes it.
    ///
    /// The undo command may be dropped: a caller that is not building an undo
    /// stack has no use for it.
    ///
    /// # Panics
    /// If `piece` or `point` is out of range.
    #[allow(
        clippy::return_self_not_must_use,
        reason = "the undo command is optional"
    )]
    pub fn apply(&self, doc: &mut Doc) -> Command {
        match *self {
            Command::MovePoint { piece, point, to } => {
                let p = &mut doc.pieces[piece].contour[point];
                let from = *p;
                *p = to;
                Command::MovePoint {
                    piece,
                    point,
                    to: from,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "moving a point stores the exact value")]

    use super::*;

    fn doc() -> Doc {
        Doc {
            pieces: vec![Piece {
                contour: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            }],
        }
    }

    #[test]
    fn apply_moves_the_point() {
        let mut d = doc();
        Command::MovePoint {
            piece: 0,
            point: 1,
            to: [2.0, 3.0],
        }
        .apply(&mut d);
        assert_eq!(d.pieces[0].contour[1], [2.0, 3.0]);
    }

    #[test]
    fn the_returned_command_restores_the_original() {
        let mut d = doc();
        let before = d.pieces[0].contour.clone();
        let undo = Command::MovePoint {
            piece: 0,
            point: 1,
            to: [2.0, 3.0],
        }
        .apply(&mut d);
        undo.apply(&mut d);
        assert_eq!(d.pieces[0].contour, before);
    }
}
