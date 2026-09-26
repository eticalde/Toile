mod curve;
mod dart;
mod elastic;
mod hang;
mod join;
mod line;
mod mannequin;
mod name;
mod notch;
mod range;
mod symmetry;
mod topology;

use curve::{set_samples, set_segment};
use dart::{add_dart, declare_dart, remove_dart, undeclare_dart};
use elastic::{add_elastic, remove_elastic, set_ratio, set_strength};
use hang::{add_hang, remove_hang, set_station};
use join::{add_seam, remove_seam};
use line::{add_line, label_line, remove_line, set_kind};
use mannequin::{
    add_mannequin, refresh_mannequin, remove_mannequin, rename_mannequin, resolve_with,
    set_measure, set_phenotype,
};
pub(crate) use name::Naming;
use name::{label_point, rename_piece, show_label};
use notch::{add_notch, move_notch, remove_notch};
use symmetry::{add_symmetry, remove_symmetry};
use topology::{add_piece, insert_node, remove_node, remove_piece};

use crate::{
    Applied, Axis, Binding, ChangeClass, Command, Doc, DocError, Grain, PieceKey, Placement,
    PointKey, VariableKey,
};

impl Command {
    /// Applies the edit and hands back the command that undoes it.
    ///
    /// # Errors
    /// `StaleKey` for a dead key and `Occupied` for a taken one,
    /// `DuplicateLabel`, `DuplicatePieceName` or `DuplicateMannequinName` for a
    /// name taken, `UnknownMeasure` for a measurement the body lacks,
    /// `BodyInUse` for the body in use, `NonFinite` for a number JSON cannot
    /// spell, `NotAStem`, `NotADay` or `NotAFingerprint` for a link Toile could
    /// not have written, `ElasticRatio` or `ElasticStrength` for a pull no
    /// elastic carries, `HangStation` for a station no body has a ring for,
    /// `NoSuchNode` for an absent node, `Sampling` for a flattening no tract
    /// takes, `Shared` for a point another piece draws with, `ShortLine` for a
    /// line through one place, any `Split…` for a run whose ends disagree on
    /// their piece, `FoldAxis` and `AlreadySymmetric` for an axis whose ends
    /// are one place and a second axis on a piece, `FlatWedge`,
    /// `ScatteredWedge` and `AlreadyDarted` for a wedge no dart can be put
    /// on, and `NotYetImplemented`.
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
            Command::SetPhenotype { mannequin, to } => set_phenotype(doc, mannequin, to),
            Command::RefreshMannequin {
                mannequin,
                values,
                phenotype,
                origin,
            } => refresh_mannequin(doc, mannequin, values, phenotype, origin),
            Command::AddMannequin {
                identity,
                mannequin,
            } => add_mannequin(doc, identity, mannequin, naming),
            Command::RemoveMannequin { mannequin } => remove_mannequin(doc, mannequin),
            Command::RenameMannequin { mannequin, to } => {
                rename_mannequin(doc, mannequin, to, naming)
            }
            Command::RenamePiece { piece, to } => rename_piece(doc, piece, to, naming),
            Command::SetGrain { piece, to } => set_grain(doc, piece, to),
            Command::PlacePiece { piece, to } => place_piece(doc, piece, to),
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
            Command::AddSeam { identity, seam } => add_seam(doc, identity, seam),
            Command::RemoveSeam { seam } => remove_seam(doc, seam),
            Command::AddElastic { identity, elastic } => add_elastic(doc, identity, elastic),
            Command::RemoveElastic { elastic } => remove_elastic(doc, elastic),
            Command::SetElasticRatio { elastic, to } => set_ratio(doc, elastic, to),
            Command::SetElasticStrength { elastic, to } => set_strength(doc, elastic, to),
            Command::AddHang { identity, hang } => add_hang(doc, identity, hang),
            Command::RemoveHang { hang } => remove_hang(doc, hang),
            Command::SetHangStation { hang, to } => set_station(doc, hang, to),
            Command::AddLine { identity, line } => add_line(doc, identity, *line),
            Command::RemoveLine { line } => remove_line(doc, line),
            Command::SetLineKind { line, to } => set_kind(doc, line, to),
            Command::LabelLine { line, to } => label_line(doc, line, to),
            Command::AddNotch {
                identity,
                notch,
                mate,
            } => add_notch(doc, identity, notch, mate),
            Command::RemoveNotch { notch } => remove_notch(doc, notch),
            Command::MoveNotch { notch, to } => move_notch(doc, notch, to),
            Command::AddSymmetry {
                identity,
                symmetry: axis,
            } => add_symmetry(doc, identity, axis),
            Command::RemoveSymmetry { symmetry } => remove_symmetry(doc, symmetry),
            Command::AddDart {
                identity,
                dart,
                wedge,
            } => add_dart(doc, identity, dart, *wedge),
            Command::RemoveDart { dart } => remove_dart(doc, dart),
            Command::DeclareDart {
                identity,
                dart,
                wedge,
            } => declare_dart(doc, identity, dart, wedge),
            Command::UndeclareDart { dart } => undeclare_dart(doc, dart),
            Command::SetPin { .. } | Command::ClearPin { .. } => Err(DocError::NotYetImplemented),
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

/// Moves a piece on the overview, or hands it back to the overview's layout.
///
/// No piece is named as touched: a step through the history re-derives and
/// re-drapes the pieces it names, and nothing is derived from where a piece
/// sits on the overview.
fn place_piece(doc: &mut Doc, piece: PieceKey, to: Option<Placement>) -> Result<Applied, DocError> {
    if let Some(placement) = to {
        placement.check()?;
    }
    let held = doc
        .pieces
        .get_mut(piece)
        .ok_or_else(|| DocError::stale(piece))?;
    let from = std::mem::replace(&mut held.placement, to);
    Ok(Applied {
        inverse: Command::PlacePiece { piece, to: from },
        touched: Vec::new(),
        class: ChangeClass::Metadata,
    })
}
