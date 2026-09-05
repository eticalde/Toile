use crate::{
    Axis, Binding, ContourNode, Dart, DartKey, DartWedge, EdgeAnchor, Grain, Identity,
    MannequinKey, Notch, NotchKey, Piece, PieceKey, Pin, PinKey, Point, PointKey, Seam, SeamKey,
    SegmentEdit, Symmetry, SymmetryKey, VariableKey,
};

/// A reversible edit to the document.
///
/// Nothing here is addressed by index: every command names what it edits by
/// key, so the undo stack survives an insertion, a deletion and their undo.
/// The inverse of a deletion carries the whole payload, the key included.
#[allow(
    missing_docs,
    reason = "each variant names its edit, and its fields name themselves"
)]
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Binds both coordinates of a point.
    MovePoint { point: PointKey, to: [Binding; 2] },
    /// Binds one coordinate of a point.
    SetBinding {
        point: PointKey,
        axis: Axis,
        to: Binding,
    },
    /// Binds a pattern variable.
    SetVariable { variable: VariableKey, to: Binding },
    /// Writes one measurement of one body.
    SetMeasure {
        mannequin: MannequinKey,
        name: String,
        to: f64,
    },
    /// Chooses the body the pattern resolves against.
    ResolveWith { mannequin: MannequinKey },
    /// Adds a node to a contour, after another node or at its head.
    InsertNode {
        piece: PieceKey,
        after: Option<PointKey>,
        identity: Identity<Point>,
        node: ContourNode,
        value: Point,
    },
    /// Takes a node out of a contour, and its point out of the document.
    RemoveNode { piece: PieceKey, node: PointKey },
    /// Changes what runs between a node and the next, handles and all.
    ///
    /// It leaves `samples` where it found it: how finely a tract is flattened
    /// is `SetSamples`, and the tool that draws a curve emits both inside one
    /// gesture — the count first, because bending a tract sampled at one
    /// point would give it handles and go on drawing its chord.
    SetSegment {
        piece: PieceKey,
        node: PointKey,
        to: SegmentEdit,
    },
    /// Changes how finely a tract is flattened, within `SAMPLES`.
    SetSamples {
        piece: PieceKey,
        node: PointKey,
        to: u16,
    },
    /// Puts a piece on the table.
    AddPiece {
        identity: Identity<Piece>,
        piece: Piece,
    },
    /// Takes a piece off the table.
    RemovePiece { piece: PieceKey },
    /// Sews two stretches of contour together.
    AddSeam {
        identity: Identity<Seam>,
        seam: Seam,
    },
    /// Unpicks a seam.
    RemoveSeam { seam: SeamKey },
    /// Marks a contour, and the facing contour with it.
    AddNotch {
        identity: Identity<Notch>,
        notch: Notch,
        mate: Option<(Identity<Notch>, Notch)>,
    },
    /// Slides a notch along its contour.
    MoveNotch { notch: NotchKey, to: EdgeAnchor },
    /// Takes a notch off a contour.
    RemoveNotch { notch: NotchKey },
    /// Cuts a dart, with the wedge it takes out of the contour.
    AddDart {
        identity: Identity<Dart>,
        dart: Dart,
        wedge: Box<DartWedge>,
    },
    /// Closes a dart back up.
    RemoveDart { dart: DartKey },
    /// Folds or mirrors a piece on an axis.
    AddSymmetry {
        identity: Identity<Symmetry>,
        symmetry: Symmetry,
    },
    /// Drops an axis of symmetry.
    RemoveSymmetry { symmetry: SymmetryKey },
    /// Renames a piece.
    RenamePiece { piece: PieceKey, to: String },
    /// Turns the grain line of a piece.
    SetGrain { piece: PieceKey, to: Grain },
    /// Names a point, or takes its name away.
    LabelPoint { point: PointKey, to: Option<String> },
    /// Decides whether the drawing shows a point's name unasked.
    ShowLabel { point: PointKey, to: bool },
    /// Holds a piece of cloth to a place in space.
    SetPin { identity: Identity<Pin>, pin: Pin },
    /// Lets it go.
    ClearPin { pin: PinKey },
}

/// What an edit costs the derivation downstream of it.
///
/// The class is the budget: a shape edit re-derives rest lengths on the spot,
/// a topology edit re-meshes off the interface thread, metadata costs nothing
/// and a simulation edit is a message to the solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeClass {
    /// The contour keeps its nodes and moves.
    Shape,
    /// The contour gains or loses nodes, or a piece does.
    Topology,
    /// Nothing the solver reads has changed.
    Metadata,
    /// Only the simulation has anything to do.
    Sim,
}

/// What applying a command left behind.
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    /// The command that undoes it.
    pub inverse: Command,
    /// The pieces the edit changed, in key order.
    pub touched: Vec<PieceKey>,
    /// What the derivation has to redo.
    pub class: ChangeClass,
}

impl Command {
    /// What this edit costs downstream.
    ///
    /// The match has no wildcard: a command added without a row here does not
    /// compile, which is what keeps the budgets honest.
    pub fn class(&self) -> ChangeClass {
        match self {
            Command::MovePoint { .. }
            | Command::SetBinding { .. }
            | Command::SetVariable { .. }
            | Command::SetMeasure { .. }
            | Command::ResolveWith { .. } => ChangeClass::Shape,
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
            | Command::RemoveSymmetry { .. } => ChangeClass::Topology,
            Command::RenamePiece { .. }
            | Command::SetGrain { .. }
            | Command::LabelPoint { .. }
            | Command::ShowLabel { .. } => ChangeClass::Metadata,
            Command::SetPin { .. } | Command::ClearPin { .. } => ChangeClass::Sim,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_a_notch_is_topology_because_it_forces_a_boundary_vertex() {
        let command = Command::MoveNotch {
            notch: NotchKey::new(0, 0),
            to: EdgeAnchor::at_node(PieceKey::new(0, 0), PointKey::new(0, 0)),
        };
        assert_eq!(command.class(), ChangeClass::Topology);
    }

    #[test]
    fn choosing_another_body_is_a_change_of_shape() {
        let command = Command::ResolveWith {
            mannequin: MannequinKey::new(1, 0),
        };
        assert_eq!(command.class(), ChangeClass::Shape);
    }

    #[test]
    fn naming_a_point_costs_the_solver_nothing() {
        let command = Command::LabelPoint {
            point: PointKey::new(0, 0),
            to: Some("cadera_lat".to_owned()),
        };
        assert_eq!(command.class(), ChangeClass::Metadata);
    }
}
