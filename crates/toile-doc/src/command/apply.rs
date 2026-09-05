mod curve;
mod name;
mod topology;

use curve::{set_samples, set_segment};
pub(crate) use name::Naming;
use name::{label_point, rename_piece, show_label};
use topology::{add_piece, insert_node, remove_node, remove_piece};

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
    /// measurement the body does not carry, `NoSuchNode` for a contour that
    /// does not run through the node named, `Occupied` for a key another point
    /// still holds, `Sampling` for a flattening no tract can be asked for,
    /// `Shared` for a point another piece still draws itself with, and
    /// `NotYetImplemented` for an edit whose tool has not been built yet.
    pub fn apply(self, doc: &mut Doc) -> Result<Applied, DocError> {
        self.apply_as(doc, Naming::Checked)
    }

    /// The same edit, told how strictly to check the names it writes.
    ///
    /// # Errors
    /// The same as `apply`.
    pub(crate) fn apply_as(self, doc: &mut Doc, naming: Naming) -> Result<Applied, DocError> {
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
            Command::RenamePiece { piece, to } => rename_piece(doc, piece, to, naming),
            Command::SetGrain { piece, to } => set_grain(doc, piece, to),
            Command::LabelPoint { point, to } => label_point(doc, point, to, naming),
            Command::ShowLabel { point, to } => show_label(doc, point, to),
            Command::SetSegment { piece, node, to } => set_segment(doc, piece, node, to),
            Command::SetSamples { piece, node, to } => set_samples(doc, piece, node, to),
            Command::InsertNode {
                piece,
                after,
                identity,
                value,
                segment,
                samples,
            } => insert_node(doc, piece, after, identity, value, segment, samples),
            Command::RemoveNode { piece, node } => remove_node(doc, piece, node),
            Command::AddPiece { identity, piece } => add_piece(doc, identity, piece, naming),
            Command::RemovePiece { piece } => remove_piece(doc, piece),
            Command::AddSeam { .. }
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
