use std::fmt::Write;

use super::super::block::{self, Room};
use super::super::drawn::{self, Drawn, Inked, Run};
use super::super::units::{BROKEN, CAPTION, DRAWN, MM_PER_CM, TITLE, beside, number};
use super::super::{cut, metric};
use super::grid::{Cloth, OVERLAP};
use super::paper::points;
use super::place::Place;
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
    /// Its handle on the paper: its letter, where its author gave it one, and
    /// its name.
    handle: String,
    /// What it says about being cut out, line by line.
    cut_out: Vec<String>,
    /// Whose measurements the pattern was drawn on, when it resolves against a
    /// body at all.
    body: Option<String>,
    /// The marks themselves, in the centimetres the document counts in.
    drawn: Drawn,
}

impl Ink {
    /// Everything a sheet of this piece carries.
    pub(super) fn new(draft: &Draft, piece: PieceKey) -> Ink {
        let held = draft.doc().pieces.get(piece);
        Ink {
            name: super::named(draft, piece),
            handle: held.map(cut::handle).unwrap_or_default(),
            cut_out: held.map(cut::says).unwrap_or_default(),
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

/// Every run of ink the piece carries besides the line it is cut on.
///
/// No colour is set here, and none anywhere else on the sheet: a printed
/// pattern is black on white, which is what a `PDF` draws with when nothing
/// asks for anything else. The screen's `theme::Theme` has no business on
/// paper, and the drawing states the same colour only because its own default
/// is not black.
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
}

/// The names the piece's nodes carry, and the box each one took.
///
/// A line's own name is not inked, which is the drawing's answer too — it hands
/// it to a title, and a title is not drawn either. The owner's waistband
/// settles it: seven belt loops two centimetres apart would print seven names
/// over each other, and what tells a cutter which loop is which is where the
/// mark is.
///
/// The boxes go back to the sheet because the words a piece says have to go
/// where the names are not: a name is a coordinate a person matches between two
/// taped sheets, and it is the one of the two he can read nowhere else.
pub(super) fn named(out: &mut String, frame: &Frame, cloth: &Cloth, ink: &Ink) -> Vec<[f64; 4]> {
    let mut taken = Vec::with_capacity(ink.drawn.names.len());
    for (label, at) in &ink.drawn.names {
        let at = labelled(frame, cloth.onto(*at), label);
        show(out, frame.point(at), CAPTION, label);
        taken.push(metric::box_of(CAPTION, at, label));
    }
    taken
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
/// The room a name needs is the room Helvetica sets it in, which the sheet
/// knows without embedding it: the font it names is one of the fourteen, so
/// `metric` has its widths.
fn labelled(frame: &Frame, at: [f64; 2], label: &str) -> [f64; 2] {
    let corner = frame.at();
    let [wide, tall] = frame.cell();
    let held = |v: f64, lo: f64, hi: f64| v >= lo && v <= hi;
    if !held(at[0], corner[0], corner[0] + wide) || !held(at[1], corner[1], corner[1] + tall) {
        return beside(at);
    }
    let [x, y] = beside(at);
    let room = metric::wide(CAPTION, label);
    [
        x.min(corner[0] + wide - room).max(corner[0]),
        y.max(corner[1] + CAPTION).min(corner[1] + tall),
    ]
}

/// What the piece says about itself: its handle, what it measures, and what its
/// author wrote about cutting it out — placed by [`block::laid`], which is
/// where the rules that place it live.
///
/// Laid against the part of its own box this sheet carries, so that every
/// sheet of a pile carries the block of every piece on it: a block left at the
/// piece's own corner would be on the first sheet alone.
///
/// On every piece, and not only where a sheet carries more than one. The legend
/// names the piece too, but it sits in the band below the cell and a tiled
/// sheet tells a person to cut along the line that band is under: by the time
/// the sheets are taped the legend is in the bin, and these are the only words
/// left on the pattern.
///
/// Answers with the box every line of it took, so the next piece on the sheet
/// can keep off them, and with nothing where the block had nowhere to go: that
/// one is left off and counted rather than printed over something.
pub(super) fn says(
    out: &mut String,
    frame: &Frame,
    cloth: &Cloth,
    ink: &Ink,
    place: &Place,
    over: &Over<'_>,
) -> Option<Vec<[f64; 4]>> {
    let [wide, tall] = cloth.size();
    // The size goes under the handle and not beside it, because a piece five
    // centimetres wide can be laid against the right margin and the two
    // together are wider than that; and into the drawing rather than into the
    // legend, because what a ruler is laid along is the piece, and a list of
    // sizes beside the square would leave a person matching three numbers to
    // three shapes.
    let mut said = vec![format!(
        "{:.1} × {:.1} cm",
        wide / MM_PER_CM,
        tall / MM_PER_CM
    )];
    said.extend(ink.cut_out.iter().cloned());
    let mut lines: Vec<(f64, &str)> = vec![(TITLE, ink.handle.as_str())];
    lines.extend(said.iter().map(|line| (CAPTION, line.as_str())));
    let room = Room {
        sheet: trimmed(frame, place),
        cloth: cloth.cut(),
        taken: over.taken,
        others: over.others,
    };
    let laid = block::laid(&lines, &room)?;
    let mut taken = Vec::with_capacity(laid.len());
    for line in laid {
        show(out, frame.point(line.at), line.size, &line.body);
        taken.push(metric::box_of(line.size, line.at, &line.body));
    }
    Some(taken)
}

/// What is already on a sheet when a piece comes to say its own words.
pub(super) struct Over<'a> {
    /// The boxes of every node name this sheet has drawn, of every piece on
    /// it: a block that dodged only its own piece's names would land on its
    /// neighbour's.
    pub(super) taken: &'a [[f64; 4]],
    /// The outlines of the other pieces the sheet carries.
    pub(super) others: &'a [&'a [[f64; 2]]],
}

/// The part of the sheet a piece's own words may be laid on: inside the clip,
/// and clear of the bands this sheet laps under its neighbours.
///
/// The clip ends at the cell, so a block that reached past the right edge or
/// low down would lose whichever lines run past it — and a line of a cutting
/// instruction half printed is worse than none, because it still reads.
///
/// A band is worse again: it is the part of the sheet a neighbour is taped
/// over, so words there end up under the neighbour's paper, and it is where the
/// crosses and the name of the joint go. On the owner's jeans his front panel's
/// handle landed on `F1-3` on five of its twelve sheets.
fn trimmed(frame: &Frame, place: &Place) -> [f64; 4] {
    let corner = frame.at();
    let [wide, tall] = frame.cell();
    let joins = place.joins;
    let band = |beside: Option<usize>| if beside.is_some() { OVERLAP } else { 0.0 };
    [
        corner[0] + band(joins.left),
        corner[1] + band(joins.above),
        corner[0] + wide - band(joins.right),
        corner[1] + tall - band(joins.below),
    ]
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
