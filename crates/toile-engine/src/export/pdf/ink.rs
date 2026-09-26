use std::fmt::Write;

use super::super::drawn::{self, Drawn, Inked, Run};
use super::super::units::{BROKEN, CAPTION, DRAWN, MM_PER_CM, TITLE, beside, number};
use super::grid::Cloth;
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
        Ink {
            name: super::named(draft, piece),
            body: draft.doc().measures().map(|set| set.name.clone()),
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
pub(super) fn marks(out: &mut String, frame: &Frame, cloth: &Cloth, ink: &Ink) {
    let runs = &ink.drawn.runs;
    let _ = writeln!(out, "{} w", number(points(DRAWN)));
    for run in runs.iter().filter(|run| !run.broken) {
        path(out, frame, cloth, run);
    }
    if runs.iter().any(|run| run.broken) {
        let _ = writeln!(
            out,
            "[{} {}] 0 d",
            number(points(BROKEN[0])),
            number(points(BROKEN[1]))
        );
        for run in runs.iter().filter(|run| run.broken) {
            path(out, frame, cloth, run);
        }
        out.push_str("[] 0 d\n");
    }
    for (label, at) in &ink.drawn.names {
        let on_the_plane = cloth.onto(*at);
        show(
            out,
            frame.point(labelled(frame, on_the_plane, label)),
            CAPTION,
            label,
        );
    }
}

/// Where a node's name goes on this sheet, kept inside the paper.
///
/// [`beside`] sets a name up and to the right of its node, which is where a
/// reader looks for it — and off the sheet for a node on the piece's own top or
/// right edge, because those sit on the cell's boundary and the clip ends
/// there. Measured on the owner's jeans, fifteen of his sixty-four names
/// reached no sheet of the file at all: they are the topmost row and the
/// rightmost column of a pile, so no neighbour held them either.
///
/// Only a node this sheet carries is moved. A name whose node is on another
/// sheet is left where it falls and clipped away, because that other sheet is
/// printing it: dragging it into the margin here would print it twice.
///
/// The room a name needs is estimated and not measured — the sheet embeds no
/// font, so nothing knows Helvetica's own tables — and what the estimate has
/// to be good enough for is keeping four characters off the margin.
fn labelled(frame: &Frame, at: [f64; 2], label: &str) -> [f64; 2] {
    let corner = frame.at();
    let [wide, tall] = frame.cell();
    let held = |v: f64, lo: f64, hi: f64| v >= lo && v <= hi;
    if !held(at[0], corner[0], corner[0] + wide) || !held(at[1], corner[1], corner[1] + tall) {
        return beside(at);
    }
    let [x, y] = beside(at);
    let room = CAPTION * 0.55 * label.chars().count() as f64;
    [
        x.min(corner[0] + wide - room).max(corner[0]),
        y.max(corner[1] + CAPTION).min(corner[1] + tall),
    ]
}

/// The piece's own name and size, where the sheet carries more than one piece.
///
/// Inside the piece's own box and not on the blank paper above it: the piece on
/// the plane's first shelf has nothing above it, so a name set in the gap would
/// be clipped off exactly the sheet that needs it most. One type height down
/// from the top of the box, so the words sit under the topmost line instead of
/// across it.
///
/// Placed at the leftmost and topmost corner of the part of the box this sheet
/// carries, which is how every sheet of a pile comes to name every piece on it:
/// a name left at the piece's own corner would be on the first sheet alone.
///
/// The size goes under the name and not beside it, because a piece five
/// centimetres wide can be laid against the right margin and the two together
/// are wider than that; and into the drawing rather than into the legend,
/// because what a ruler is laid along is the piece, and a list of sizes beside
/// the square would leave a person matching three numbers to three shapes.
pub(super) fn named(out: &mut String, frame: &Frame, cloth: &Cloth, ink: &Ink) {
    let [across, down] = cloth.place();
    let [wide, tall] = cloth.size();
    let at = frame.at();
    let corner = [across.max(at[0]) + CAPTION / 2.0, down.max(at[1]) + TITLE];
    show(
        out,
        frame.point(corner),
        TITLE,
        &format!("«{}»", ink.name()),
    );
    let under = [corner[0], corner[1] + CAPTION * 1.5];
    let size = format!("{:.1} × {:.1} cm", wide / MM_PER_CM, tall / MM_PER_CM);
    show(out, frame.point(under), CAPTION, &size);
}

/// One run as an open path on the sheet.
fn path(out: &mut String, frame: &Frame, cloth: &Cloth, run: &Run) {
    for (rank, &at) in run.at.iter().enumerate() {
        let [x, y] = frame.point(cloth.onto(at));
        let verb = if rank == 0 { 'm' } else { 'l' };
        let _ = writeln!(out, "{} {} {verb}", number(x), number(y));
    }
    out.push_str("S\n");
}
