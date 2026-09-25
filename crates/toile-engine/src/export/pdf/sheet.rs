use std::fmt::Write;

use super::super::units::{CUT, MARGIN, number};
use super::grid::Grid;
use super::ink::Ink;
use super::paper::{Paper, points};
use super::place::Place;
use super::{grid, ink, join, legend};

/// Where one sheet's drawing sits on its sheet of paper.
pub(super) struct Frame {
    paper: Paper,
    /// Where this sheet's own drawing starts on the tiled plane, in
    /// millimetres. A piece that takes one sheet starts at the plane's own
    /// corner; every sheet after that starts one step further along it.
    at: [f64; 2],
    /// How much of this sheet the pattern may be drawn on, in millimetres.
    cell: [f64; 2],
}

impl Frame {
    /// The sheet that carries the tiled plane from `at` onward.
    pub(super) fn new(paper: Paper, at: [f64; 2]) -> Frame {
        Frame {
            paper,
            at,
            cell: grid::cell(paper),
        }
    }

    /// What the sheet is made of.
    pub(super) fn paper(&self) -> Paper {
        self.paper
    }

    /// How much of the sheet the pattern may take, in millimetres.
    pub(super) fn cell(&self) -> [f64; 2] {
        self.cell
    }

    /// A place on the page in points, from a place on the tiled plane.
    ///
    /// The whole transform in one line, and the only place it happens: the
    /// plane the taped sheets make is measured in millimetres from its own
    /// top left corner, and this sheet carries the part of it that starts
    /// at `at`.
    pub(super) fn point(&self, [x, y]: [f64; 2]) -> [f64; 2] {
        self.page(MARGIN + x - self.at[0], MARGIN + y - self.at[1])
    }

    /// A place on the page in points, from one measured in millimetres down and
    /// across from the sheet's own top left corner.
    ///
    /// The one place a page's own reckoning is turned around: paper is laid out
    /// downward from the corner a person reads from, and a page is measured
    /// upward from the corner a printer starts at.
    pub(super) fn page(&self, across: f64, down: f64) -> [f64; 2] {
        [points(across), points(self.paper.height_mm - down)]
    }
}

/// One sheet's content stream: the part of the piece this sheet carries, what a
/// person joins it by, the square a ruler is laid on, and the Spanish that says
/// what to do with all three.
///
/// The piece's own ink is written inside the clip and the rest of the sheet
/// outside it, so the cut weight the clip opens with is still the current one
/// when the square is stroked after it closes.
pub(super) fn content(frame: &Frame, grid: &Grid, ink: &Ink, place: &Place) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{} w", number(points(CUT)));
    out.push_str("q\n");
    clip(&mut out, frame);
    cut(&mut out, frame, grid.cut());
    ink::marks(&mut out, frame, grid, ink);
    out.push_str("Q\n");
    legend::square(&mut out);
    join::marks(&mut out, frame, place);
    legend::says(&mut out, frame, ink, grid.size(), place);
    out
}

/// The page held to its own cell.
///
/// The piece is written whole on every sheet and cut to the sheet here, which
/// is what keeps one line of the pattern one line across a joint: the two
/// halves are the same path, stopped at the same place from opposite sides.
/// Without it a sheet would carry the rest of the piece over its neighbour's
/// paper and over the band that proves the scale.
///
/// Opened by half the cut weight on every side, because a stroke is centred on
/// its path: a cut line running along the cell's own edge — which is where
/// every piece's topmost line falls — would lose its outer half to a clip that
/// ended there and render at the weight this module reserves for a mark,
/// inverting the one distinction a sheet has to make at a glance. What leaks is
/// 0.15 mm onto paper the next sheet is taped over anyway.
fn clip(out: &mut String, frame: &Frame) {
    let [wide, tall] = frame.cell();
    let bleed = CUT / 2.0;
    let [x, y] = frame.page(MARGIN - bleed, MARGIN + tall + bleed);
    let _ = writeln!(
        out,
        "{} {} {} {} re W n",
        number(x),
        number(y),
        number(points(wide + CUT)),
        number(points(tall + CUT))
    );
}

/// The cut line: one closed path, in contour order.
fn cut(out: &mut String, frame: &Frame, plane: &[[f64; 2]]) {
    for (rank, &at) in plane.iter().enumerate() {
        let [x, y] = frame.point(at);
        let verb = if rank == 0 { 'm' } else { 'l' };
        let _ = writeln!(out, "{} {} {verb}", number(x), number(y));
    }
    out.push_str("h S\n");
}
