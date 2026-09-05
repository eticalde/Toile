use crate::{Applied, ChangeClass, Command, Doc, DocError, PieceKey, PointKey};

/// How strictly a command checks a name against the ones already taken.
///
/// An undo is one transaction, not a sequence of edits. A gesture that swapped
/// two names replays its inverses through a moment where both points want the
/// same one, and a per-command check would refuse it and strand the entry on
/// the stack for good. Every state the stack holds was accepted once, so a
/// collision seen while it is being replayed belongs to the order of the
/// replay and not to the state being restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Naming {
    /// A fresh edit: a name another item already shows is refused.
    Checked,
    /// A step of the undo stack: the collisions on the way through are let by.
    Restoring,
}

/// Renames a piece, refusing a name another piece carries.
pub(crate) fn rename_piece(
    doc: &mut Doc,
    piece: PieceKey,
    to: String,
    naming: Naming,
) -> Result<Applied, DocError> {
    if naming == Naming::Checked
        && let Some(other) = doc.piece_named(&to)
        && other != piece
    {
        return Err(DocError::DuplicatePieceName(to));
    }
    let held = doc
        .pieces
        .get_mut(piece)
        .ok_or_else(|| DocError::stale(piece))?;
    let from = std::mem::replace(&mut held.name, to);
    Ok(Applied {
        inverse: Command::RenamePiece { piece, to: from },
        touched: vec![piece],
        class: ChangeClass::Metadata,
    })
}

/// Names a point, refusing a name another point of the same piece shows.
///
/// A collision is an error the user can act on, never a name with a number
/// stuck on the end of it. Clearing a name collides too: the point falls back
/// to its automatic one, which another point may hold explicitly.
pub(crate) fn label_point(
    doc: &mut Doc,
    point: PointKey,
    to: Option<String>,
    naming: Naming,
) -> Result<Applied, DocError> {
    let touched = doc.pieces_citing(point);
    if doc.points.get(point).is_none() {
        return Err(DocError::stale(point));
    }
    if naming == Naming::Checked {
        taken(doc, point, to.as_ref(), &touched)?;
    }
    let held = doc
        .points
        .get_mut(point)
        .ok_or_else(|| DocError::stale(point))?;
    let from = std::mem::replace(&mut held.label, to);
    Ok(Applied {
        inverse: Command::LabelPoint { point, to: from },
        touched,
        class: ChangeClass::Metadata,
    })
}

/// Whether any piece running through `point` already shows the name it wants.
fn taken(
    doc: &Doc,
    point: PointKey,
    to: Option<&String>,
    touched: &[PieceKey],
) -> Result<(), DocError> {
    for &piece in touched {
        let Some(shown) = to.cloned().or_else(|| doc.automatic_label(piece, point)) else {
            continue;
        };
        if doc
            .shows_label(piece, &shown)
            .is_some_and(|other| other != point)
        {
            return Err(DocError::DuplicateLabel(shown));
        }
    }
    Ok(())
}

/// Decides whether the drawing writes a point's name unasked.
pub(crate) fn show_label(doc: &mut Doc, point: PointKey, to: bool) -> Result<Applied, DocError> {
    let touched = doc.pieces_citing(point);
    let held = doc
        .points
        .get_mut(point)
        .ok_or_else(|| DocError::stale(point))?;
    let from = std::mem::replace(&mut held.label_visible, to);
    Ok(Applied {
        inverse: Command::ShowLabel { point, to: from },
        touched,
        class: ChangeClass::Metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block;

    /// The block's front, one of its nodes, and the automatic name that node
    /// would fall back to if its written one were cleared.
    fn front() -> (Doc, PieceKey, PointKey, PointKey, String) {
        let doc = block::trouser_front();
        let piece = doc.piece_named(block::FRONT).expect("the block draws one");
        let waist = doc
            .shows_label(piece, "cintura_lat")
            .expect("the block names it");
        let hip = doc
            .shows_label(piece, "cadera_lat")
            .expect("the block names it");
        let automatic = doc
            .automatic_label(piece, waist)
            .expect("the piece runs through it");
        (doc, piece, waist, hip, automatic)
    }

    #[test]
    fn clearing_a_name_is_refused_when_the_automatic_one_is_taken() {
        let (mut doc, _, waist, hip, automatic) = front();
        Command::LabelPoint {
            point: hip,
            to: Some(automatic.clone()),
        }
        .apply(&mut doc)
        .expect("the automatic name is free while the waist writes its own");

        let fault = Command::LabelPoint {
            point: waist,
            to: None,
        }
        .apply(&mut doc)
        .unwrap_err();
        assert_eq!(fault, DocError::DuplicateLabel(automatic));
    }

    #[test]
    fn clearing_a_name_falls_back_to_the_automatic_one() {
        let (mut doc, piece, waist, _, automatic) = front();
        Command::LabelPoint {
            point: waist,
            to: None,
        }
        .apply(&mut doc)
        .expect("nothing else shows that name");
        assert_eq!(doc.label_of(piece, waist), Some(automatic));
    }

    #[test]
    fn a_name_being_restored_is_written_over_a_collision() {
        let (mut doc, piece, waist, hip, _) = front();
        let refused = label_point(
            &mut doc,
            hip,
            Some("cintura_lat".to_owned()),
            Naming::Checked,
        );
        assert_eq!(
            refused,
            Err(DocError::DuplicateLabel("cintura_lat".to_owned()))
        );
        label_point(
            &mut doc,
            hip,
            Some("cintura_lat".to_owned()),
            Naming::Restoring,
        )
        .expect("a replay of the stack passes through collisions");
        assert_eq!(doc.label_of(piece, hip).as_deref(), Some("cintura_lat"));
        assert_eq!(doc.label_of(piece, waist).as_deref(), Some("cintura_lat"));
    }
}
