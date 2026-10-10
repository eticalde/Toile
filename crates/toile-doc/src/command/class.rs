use crate::{Command, PieceKey};

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
            // A refresh carries a phenotype too, but it is priced by its tape,
            // which is what the pattern resolves against.
            | Command::RefreshMannequin { .. }
            | Command::ResolveWith { .. }
            // The whole of an elastic is a rest length. The drawing does not
            // move and no node is gained, so the cloth of the piece is
            // re-derived where it stands and the drape carries on.
            | Command::AddElastic { .. }
            | Command::RemoveElastic { .. }
            | Command::SetElasticRatio { .. }
            | Command::SetElasticStrength { .. }
            // And a hang for the same reason as an elastic, though it writes
            // no rest length: the run of cloth it holds is read off the
            // resolved contour, so it is re-read where the piece stands and
            // the drape carries on.
            | Command::AddHang { .. }
            | Command::RemoveHang { .. }
            | Command::SetHangStation { .. }
            | Command::SetHangHeading { .. } => ChangeClass::Shape,
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
            // A declaration gains no node, so nothing forces a remesh by
            // itself — but it issues the seam that shuts the wedge, and a seam
            // is paired where the cloth is built. It is priced with the seam it
            // brings, which is the row `AddSeam` already sits in.
            | Command::DeclareDart { .. }
            | Command::UndeclareDart { .. }
            | Command::AddSymmetry { .. }
            | Command::RemoveSymmetry { .. } => ChangeClass::Topology,
            // A body that is added, renamed or removed is never the one the
            // pattern resolves against at that moment, so no formula moves.
            Command::AddMannequin { .. }
            | Command::RemoveMannequin { .. }
            | Command::RenameMannequin { .. }
            | Command::RenamePiece { .. }
            | Command::SetGrain { .. }
            // What a piece says about being cut out is read off the paper and
            // by nothing else: no contour is offset by the allowance, and no
            // mesh is cut twice because the count says two.
            | Command::SetSeamAllowance { .. }
            | Command::SetQuantity { .. }
            | Command::SetLetter { .. }
            | Command::SetLabels { .. }
            // Where a piece sits on the overview is layout: no contour, mesh
            // or drape is derived from it.
            | Command::PlacePiece { .. }
            // An internal line is drawn on the paper and nothing reads it
            // back: not the contour, not the mesh, not the drape. A curved
            // span of one hangs on handles that are points of the document,
            // and no contour cites them, so nothing resolves them either.
            | Command::AddLine { .. }
            | Command::RemoveLine { .. }
            | Command::SetLineKind { .. }
            | Command::LabelLine { .. }
            | Command::LabelPoint { .. }
            | Command::ShowLabel { .. } => ChangeClass::Metadata,
            // The phenotype shapes the body and nothing the pattern resolves.
            // Nothing drapes on that body yet — the cloth still falls on the
            // sphere — so today no consumer answers this class for it.
            Command::SetPhenotype { .. } | Command::SetPin { .. } | Command::ClearPin { .. } => {
                ChangeClass::Sim
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EdgeAnchor, LineKey, LineKind, MannequinKey, NotchKey, PointKey};

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

    /// The cloth does not read an internal line, so the derivation owes it
    /// nothing — the same price as moving a piece on the overview.
    #[test]
    fn drawing_a_line_on_a_piece_costs_the_derivation_nothing() {
        let command = Command::SetLineKind {
            line: LineKey::new(0, 0),
            to: LineKind::Fold,
        };
        assert_eq!(command.class(), ChangeClass::Metadata);
    }
}
