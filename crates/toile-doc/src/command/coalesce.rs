use crate::{Axis, Command, MannequinKey, NotchKey, PieceKey, PointKey, VariableKey};

/// Whether one edit stands for an earlier edit of the same gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coalesced {
    /// The later edit writes the same field, so it replaces the earlier one.
    Replaces,
    /// Two steps of the gesture, and the entry keeps both.
    Separate,
}

/// The field an edit overwrites, when a later edit can stand for it.
///
/// Only a whole-field write appears here. An edit that creates or destroys an
/// entity never folds: two of them are two things done, not one thing done
/// twice, and folding them would strand the key the second one issued.
#[derive(PartialEq)]
enum Field<'a> {
    Position(PointKey),
    Coordinate(PointKey, Axis),
    Variable(VariableKey),
    Measure(MannequinKey, &'a str),
    Body,
    Run(PieceKey, PointKey),
    Samples(PieceKey, PointKey),
    NotchPlace(NotchKey),
    PieceName(PieceKey),
    Grain(PieceKey),
    Label(PointKey),
    LabelShown(PointKey),
}

impl Command {
    /// Whether this edit folds onto `previous` inside one gesture.
    ///
    /// Folding is declared, never guessed: a drag emits one `MovePoint` per
    /// frame and the gesture keeps one, but two edits that only look alike
    /// stay two. The undo entry keeps the first inverse either way, so undo
    /// goes back to where the gesture started rather than to its last frame.
    pub fn coalesce_onto(&self, previous: &Command) -> Coalesced {
        match (self.field(), previous.field()) {
            (Some(field), Some(before)) if field == before => Coalesced::Replaces,
            _ => Coalesced::Separate,
        }
    }

    /// The field this edit writes, if a later edit can stand for it.
    ///
    /// The match has no wildcard: a command added without a row here does not
    /// compile, so nothing folds by accident.
    fn field(&self) -> Option<Field<'_>> {
        match self {
            Command::MovePoint { point, .. } => Some(Field::Position(*point)),
            Command::SetBinding { point, axis, .. } => Some(Field::Coordinate(*point, *axis)),
            Command::SetVariable { variable, .. } => Some(Field::Variable(*variable)),
            Command::SetMeasure {
                mannequin, name, ..
            } => Some(Field::Measure(*mannequin, name)),
            Command::ResolveWith { .. } => Some(Field::Body),
            Command::SetSegment { piece, node, .. } => Some(Field::Run(*piece, *node)),
            Command::SetSamples { piece, node, .. } => Some(Field::Samples(*piece, *node)),
            Command::MoveNotch { notch, .. } => Some(Field::NotchPlace(*notch)),
            Command::RenamePiece { piece, .. } => Some(Field::PieceName(*piece)),
            Command::SetGrain { piece, .. } => Some(Field::Grain(*piece)),
            Command::LabelPoint { point, .. } => Some(Field::Label(*point)),
            Command::ShowLabel { point, .. } => Some(Field::LabelShown(*point)),
            Command::InsertNode { .. }
            | Command::RemoveNode { .. }
            | Command::AddPiece { .. }
            | Command::RemovePiece { .. }
            | Command::AddSeam { .. }
            | Command::RemoveSeam { .. }
            | Command::AddNotch { .. }
            | Command::RemoveNotch { .. }
            | Command::AddDart { .. }
            | Command::RemoveDart { .. }
            | Command::AddSymmetry { .. }
            | Command::RemoveSymmetry { .. }
            | Command::SetPin { .. }
            | Command::ClearPin { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Binding, Grain, Identity, Pin, PinKey};

    fn move_to(point: PointKey, x: f64) -> Command {
        Command::MovePoint {
            point,
            to: [Binding::literal(x), Binding::literal(0.0)],
        }
    }

    #[test]
    fn two_frames_of_one_drag_fold_into_one() {
        let point = PointKey::new(3, 0);
        assert_eq!(
            move_to(point, 2.0).coalesce_onto(&move_to(point, 1.0)),
            Coalesced::Replaces
        );
    }

    #[test]
    fn a_drag_of_another_point_does_not_fold() {
        let first = move_to(PointKey::new(3, 0), 1.0);
        let second = move_to(PointKey::new(4, 0), 1.0);
        assert_eq!(second.coalesce_onto(&first), Coalesced::Separate);
    }

    #[test]
    fn the_two_coordinates_of_a_point_are_two_fields() {
        let point = PointKey::new(3, 0);
        let coordinate = |axis| Command::SetBinding {
            point,
            axis,
            to: Binding::literal(1.0),
        };
        assert_eq!(
            coordinate(Axis::X).coalesce_onto(&coordinate(Axis::X)),
            Coalesced::Replaces
        );
        assert_eq!(
            coordinate(Axis::Y).coalesce_onto(&coordinate(Axis::X)),
            Coalesced::Separate
        );
        assert_eq!(
            coordinate(Axis::X).coalesce_onto(&move_to(point, 1.0)),
            Coalesced::Separate
        );
    }

    #[test]
    fn two_measurements_of_one_body_are_two_fields() {
        let mannequin = MannequinKey::new(0, 0);
        let measure = |name: &str| Command::SetMeasure {
            mannequin,
            name: name.to_owned(),
            to: 84.0,
        };
        assert_eq!(
            measure("cintura").coalesce_onto(&measure("cintura")),
            Coalesced::Replaces
        );
        assert_eq!(
            measure("cadera").coalesce_onto(&measure("cintura")),
            Coalesced::Separate
        );
    }

    #[test]
    fn typing_a_name_folds_into_one_rename() {
        let rename = |to: &str| Command::RenamePiece {
            piece: PieceKey::new(0, 0),
            to: to.to_owned(),
        };
        assert_eq!(
            rename("Delanter").coalesce_onto(&rename("Delante")),
            Coalesced::Replaces
        );
        let grain = Command::SetGrain {
            piece: PieceKey::new(0, 0),
            to: Grain::Angle(0.0),
        };
        assert_eq!(grain.coalesce_onto(&rename("Delante")), Coalesced::Separate);
    }

    #[test]
    fn an_edit_that_makes_an_entity_never_folds() {
        let pin = |key| Command::SetPin {
            identity: Identity::Restored(key),
            pin: Pin {
                piece: PieceKey::new(0, 0),
                rest: [0.0, 0.0],
                to: [0.0, 0.0, 0.0],
            },
        };
        let key = PinKey::new(0, 0);
        assert_eq!(pin(key).coalesce_onto(&pin(key)), Coalesced::Separate);
    }
}
