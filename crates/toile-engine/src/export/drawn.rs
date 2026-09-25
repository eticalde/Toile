/// Where a place anchored to a contour falls, and which way it runs there.
mod anchor;
/// The direction the warp runs, arrowed at both ends.
mod grain;
/// The lines a piece is drawn with and not cut on.
mod line;
/// The marks a notch cuts a contour with.
mod notch;

use crate::draft::{Cloth, Draft, PieceKey};

/// One run of ink a sheet draws, in centimetres with y downward.
pub(super) struct Run {
    /// The places it passes through.
    pub(super) at: Vec<[f64; 2]>,
    /// Whether it is drawn broken.
    pub(super) broken: bool,
    /// The name its author gave it, when it carries one.
    pub(super) label: Option<String>,
}

impl Run {
    /// A run drawn whole and carrying no name, which is what a mark is.
    fn mark(at: Vec<[f64; 2]>) -> Run {
        Run {
            at,
            broken: false,
            label: None,
        }
    }
}

/// Everything a sheet draws of one piece apart from the line it is cut on.
pub(super) struct Drawn {
    /// Every run of ink, in the order a sheet inks them.
    pub(super) runs: Vec<Run>,
    /// The name each node carries, at the place that node resolved to.
    pub(super) names: Vec<(String, [f64; 2])>,
    /// How much of the pattern this is.
    pub(super) inked: Inked,
}

/// How much of a piece reached a sheet.
///
/// Counted the way a person counts it and not the way a sheet draws it: a mark
/// on a piece drawn against a fold is inked on both halves and is one mark, and
/// a triple notch is one notch drawn with three.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Inked {
    /// How many lines the piece is drawn with.
    pub lines: usize,
    /// How many notches mark its contour.
    pub notches: usize,
    /// How many of its nodes carry a name.
    pub names: usize,
}

/// Everything drawn on a piece besides its cut line, in the centimetres the
/// document counts in.
///
/// Worked out here and not in either sheet, so that the drawing and the printed
/// page carry the same marks in the same places and only the dialect they are
/// written in differs.
pub(super) fn of(draft: &Draft, piece: PieceKey) -> Drawn {
    let cloth = draft.cloth(piece);
    let mut runs = Vec::new();
    let mut inked = Inked::default();
    for run in line::runs(draft, piece) {
        inked.lines += 1;
        both(&mut runs, run, cloth);
    }
    for notch in notch::runs(draft, piece) {
        inked.notches += 1;
        for mark in notch {
            both(&mut runs, mark, cloth);
        }
    }
    runs.extend(grain::runs(draft, piece));
    let names = names(draft, piece);
    inked.names = names.len();
    Drawn { runs, names, inked }
}

/// One run, and on a piece drawn against a fold its mirror image as well.
///
/// The sheet is the whole cloth, and a mark drawn on half of it is a mark the
/// cutter finds on one side of a piece that carries it on both.
fn both(runs: &mut Vec<Run>, run: Run, cloth: Option<&Cloth>) {
    let mirrored = cloth.map(|cloth| Run {
        at: run.at.iter().map(|&at| cloth.mirror(at)).collect(),
        broken: run.broken,
        label: run.label.clone(),
    });
    runs.push(run);
    runs.extend(mirrored);
}

/// The names a piece's nodes carry, each at the place its own node resolved to.
///
/// The names go beside the nodes and not beside the flattening, so that a
/// curved tract does not scatter a label over every sample it was cut into.
fn names(draft: &Draft, piece: PieceKey) -> Vec<(String, [f64; 2])> {
    draft
        .points_cm(piece)
        .iter()
        .filter_map(|&(key, at)| Some((draft.doc().label_of(piece, key)?, at)))
        .collect()
}
