use std::fmt::Write;

use super::super::drawn::{self, Drawn, Inked, Run};
use super::super::units::{BROKEN, CAPTION, DRAWN, beside, number};
use super::grid::Grid;
use super::paper::points;
use super::sheet::Frame;
use super::text::show;
use crate::draft::{Draft, PieceKey};

/// Everything one piece puts on paper apart from the line it is cut on.
///
/// Worked out once for the piece and inked again on every sheet of it: a
/// trouser leg takes a dozen, and neither the flattening behind these runs nor
/// the walk along the contour behind these marks depends on which sheet is
/// being written.
pub(super) struct Ink {
    /// What the pattern calls the piece.
    name: String,
    /// Whose measurements the pattern was drawn on, when it resolves against a
    /// body at all.
    body: Option<String>,
    /// The marks themselves, in the centimetres the document counts in.
    drawn: Drawn,
}

impl Ink {
    /// Everything a sheet of this piece carries.
    pub(super) fn new(draft: &Draft, piece: PieceKey) -> Ink {
        let doc = draft.doc();
        Ink {
            name: doc
                .pieces
                .get(piece)
                .map_or_else(String::new, |held| held.name.clone()),
            body: doc.measures().map(|set| set.name.clone()),
            drawn: drawn::of(draft, piece),
        }
    }

    /// What the pattern calls the piece.
    pub(super) fn name(&self) -> &str {
        &self.name
    }

    /// Whose measurements it was drawn on.
    pub(super) fn body(&self) -> Option<&str> {
        self.body.as_deref()
    }

    /// How much of the piece reached the paper.
    pub(super) fn inked(&self) -> Inked {
        self.drawn.inked
    }
}

/// The piece's own marks on one sheet: every run of ink, and the names its
/// nodes carry.
///
/// No colour is set here, and none anywhere else on the sheet: a printed
/// pattern is black on white, which is what a `PDF` draws with when nothing
/// asks for anything else. The screen's `theme::Theme` has no business on
/// paper, and the drawing states the same colour only because its own default
/// is not black.
///
/// A line's own name is not inked, which is the drawing's answer too — it hands
/// it to a title, and a title is not drawn either. The owner's waistband
/// settles it: seven belt loops two centimetres apart would print seven names
/// over each other, and what tells a cutter which loop is which is where the
/// mark is.
///
/// Whole runs first and broken ones after, so the dashes are turned on once for
/// all of them and off again, and the sheet says the same bytes twice over.
pub(super) fn marks(out: &mut String, frame: &Frame, grid: &Grid, ink: &Ink) {
    let runs = &ink.drawn.runs;
    let _ = writeln!(out, "{} w", number(points(DRAWN)));
    for run in runs.iter().filter(|run| !run.broken) {
        path(out, frame, grid, run);
    }
    if runs.iter().any(|run| run.broken) {
        let _ = writeln!(
            out,
            "[{} {}] 0 d",
            number(points(BROKEN[0])),
            number(points(BROKEN[1]))
        );
        for run in runs.iter().filter(|run| run.broken) {
            path(out, frame, grid, run);
        }
        out.push_str("[] 0 d\n");
    }
    for (label, at) in &ink.drawn.names {
        show(out, frame.point(beside(grid.onto(*at))), CAPTION, label);
    }
}

/// One run as an open path on the sheet.
fn path(out: &mut String, frame: &Frame, grid: &Grid, run: &Run) {
    for (rank, &at) in run.at.iter().enumerate() {
        let [x, y] = frame.point(grid.onto(at));
        let verb = if rank == 0 { 'm' } else { 'l' };
        let _ = writeln!(out, "{} {} {verb}", number(x), number(y));
    }
    out.push_str("S\n");
}
