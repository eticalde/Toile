use crate::{
    Applied, Axis, Binding, ChangeClass, Command, Doc, DocError, Grain, MannequinKey, PieceKey,
    PointKey, VariableKey,
};

impl Command {
    /// Applies the edit and hands back the command that undoes it.
    ///
    /// # Errors
    /// `DocError::StaleKey` for a key that names nothing, `DuplicateLabel` or
    /// `DuplicatePieceName` for a name already taken, `UnknownMeasure` for a
    /// measurement the body does not carry, and `NotYetImplemented` for an
    /// edit whose tool has not been built yet.
    pub fn apply(self, doc: &mut Doc) -> Result<Applied, DocError> {
        match self {
            Command::MovePoint { point, to } => move_point(doc, point, to),
            Command::SetBinding { point, axis, to } => set_binding(doc, point, axis, to),
            Command::SetVariable { variable, to } => set_variable(doc, variable, to),
            Command::SetMeasure {
                mannequin,
                name,
                to,
            } => set_measure(doc, mannequin, name, to),
            Command::ResolveWith { mannequin } => resolve_with(doc, mannequin),
            Command::RenamePiece { piece, to } => rename_piece(doc, piece, to),
            Command::SetGrain { piece, to } => set_grain(doc, piece, to),
            Command::LabelPoint { point, to } => label_point(doc, point, to),
            Command::ShowLabel { point, to } => show_label(doc, point, to),
            Command::InsertNode { .. }
            | Command::RemoveNode { .. }
            | Command::SetSegment { .. }
            | Command::SetSamples { .. }
            | Command::AddPiece { .. }
            | Command::RemovePiece { .. }
            | Command::AddSeam { .. }
            | Command::RemoveSeam { .. }
            | Command::AddNotch { .. }
            | Command::MoveNotch { .. }
            | Command::RemoveNotch { .. }
            | Command::AddDart { .. }
            | Command::RemoveDart { .. }
            | Command::AddSymmetry { .. }
            | Command::RemoveSymmetry { .. }
            | Command::SetPin { .. }
            | Command::ClearPin { .. } => Err(DocError::NotYetImplemented),
        }
    }
}

fn move_point(doc: &mut Doc, point: PointKey, to: [Binding; 2]) -> Result<Applied, DocError> {
    let touched = doc.pieces_citing(point);
    let held = doc
        .points
        .get_mut(point)
        .ok_or_else(|| DocError::stale(point))?;
    let [x, y] = to;
    let from = [
        std::mem::replace(&mut held.x, x),
        std::mem::replace(&mut held.y, y),
    ];
    Ok(Applied {
        inverse: Command::MovePoint { point, to: from },
        touched,
        class: ChangeClass::Shape,
    })
}

fn set_binding(
    doc: &mut Doc,
    point: PointKey,
    axis: Axis,
    to: Binding,
) -> Result<Applied, DocError> {
    let touched = doc.pieces_citing(point);
    let held = doc
        .points
        .get_mut(point)
        .ok_or_else(|| DocError::stale(point))?;
    let from = std::mem::replace(held.binding_mut(axis), to);
    Ok(Applied {
        inverse: Command::SetBinding {
            point,
            axis,
            to: from,
        },
        touched,
        class: ChangeClass::Shape,
    })
}

fn set_variable(doc: &mut Doc, variable: VariableKey, to: Binding) -> Result<Applied, DocError> {
    let touched = doc.piece_keys();
    let held = doc
        .variables
        .get_mut(variable)
        .ok_or_else(|| DocError::stale(variable))?;
    let from = std::mem::replace(&mut held.value, to);
    Ok(Applied {
        inverse: Command::SetVariable { variable, to: from },
        touched,
        class: ChangeClass::Shape,
    })
}

/// Writes a measurement the body already carries.
///
/// Which measurements a body has is the mannequin's own business; an edit that
/// could introduce one could not be undone by another edit of the same shape.
fn set_measure(
    doc: &mut Doc,
    mannequin: MannequinKey,
    name: String,
    to: f64,
) -> Result<Applied, DocError> {
    let touched = if mannequin == doc.resolve_with {
        doc.piece_keys()
    } else {
        Vec::new()
    };
    let set = doc
        .mannequins
        .get_mut(mannequin)
        .ok_or_else(|| DocError::stale(mannequin))?;
    let slot = set
        .values
        .get_mut(&name)
        .ok_or_else(|| DocError::UnknownMeasure(name.clone()))?;
    let from = std::mem::replace(slot, to);
    Ok(Applied {
        inverse: Command::SetMeasure {
            mannequin,
            name,
            to: from,
        },
        touched,
        class: ChangeClass::Shape,
    })
}

fn resolve_with(doc: &mut Doc, mannequin: MannequinKey) -> Result<Applied, DocError> {
    if doc.mannequins.get(mannequin).is_none() {
        return Err(DocError::stale(mannequin));
    }
    let from = std::mem::replace(&mut doc.resolve_with, mannequin);
    Ok(Applied {
        inverse: Command::ResolveWith { mannequin: from },
        touched: doc.piece_keys(),
        class: ChangeClass::Shape,
    })
}

fn rename_piece(doc: &mut Doc, piece: PieceKey, to: String) -> Result<Applied, DocError> {
    if let Some(other) = doc.piece_named(&to)
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

fn set_grain(doc: &mut Doc, piece: PieceKey, to: Grain) -> Result<Applied, DocError> {
    let held = doc
        .pieces
        .get_mut(piece)
        .ok_or_else(|| DocError::stale(piece))?;
    let from = std::mem::replace(&mut held.grain, to);
    Ok(Applied {
        inverse: Command::SetGrain { piece, to: from },
        touched: vec![piece],
        class: ChangeClass::Metadata,
    })
}

/// Names a point, refusing a name another point of the same piece shows.
///
/// A collision is an error the user can act on, never a name with a number
/// stuck on the end of it. Clearing a name collides too: the point falls back
/// to its automatic one, which another point may hold explicitly.
fn label_point(doc: &mut Doc, point: PointKey, to: Option<String>) -> Result<Applied, DocError> {
    let touched = doc.pieces_citing(point);
    if doc.points.get(point).is_none() {
        return Err(DocError::stale(point));
    }
    for &piece in &touched {
        let Some(shown) = to.clone().or_else(|| doc.automatic_label(piece, point)) else {
            continue;
        };
        if doc
            .shows_label(piece, &shown)
            .is_some_and(|other| other != point)
        {
            return Err(DocError::DuplicateLabel(shown));
        }
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

fn show_label(doc: &mut Doc, point: PointKey, to: bool) -> Result<Applied, DocError> {
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
}
