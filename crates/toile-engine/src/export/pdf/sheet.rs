use std::fmt::Write;

use super::super::units::{CUT, DRAWN, MARGIN, number};
use super::grid::Grid;
use super::ink::{Ink, Over};
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

    /// Where this sheet's own drawing starts on the tiled plane, in
    /// millimetres.
    pub(super) fn at(&self) -> [f64; 2] {
        self.at
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

/// One sheet's content stream: the part of the pile this sheet carries, what a
/// person joins it by, the square a ruler is laid on, and the Spanish that says
/// what to do with all three.
///
/// The pieces' own ink is written inside the clip and the rest of the sheet
/// outside it, so the cut weight the clip opens with is still the current one
/// when the square is stroked after it closes.
///
/// Every cut line first and the marks after, so that the weight a cut line is
/// stroked at is set once for all of them: the one distinction a sheet has to
/// make at a glance is the line to cut from everything else, and on a sheet
/// carrying two pieces it has to make it twice.
///
/// Every piece says its own handle and how it is cut out beside its own
/// outline, whether it shares the sheet or not — the legend says some of it
/// too, and the legend is in the band a tiled sheet is trimmed of. How many of
/// those blocks the sheet had no room for goes back with the stream.
pub(super) fn content(frame: &Frame, grid: &Grid, inks: &[Ink], place: &Place) -> (String, usize) {
    let mut out = String::new();
    let _ = writeln!(out, "{} w", number(points(CUT)));
    out.push_str("q\n");
    clip(&mut out, frame);
    for cloth in grid.cloths() {
        cut(&mut out, frame, cloth.cut());
    }
    // Every piece's marks before any piece's words, and not one piece whole at
    // a time: a block has to give way to the node names of this cell, and
    // names it is written before are names it cannot see.
    let mut taken = Vec::new();
    for (cloth, ink) in grid.cloths().iter().zip(inks) {
        ink::marks(&mut out, frame, cloth, ink);
        taken.extend(ink::named(&mut out, frame, cloth, ink));
    }
    let unsaid = words(&mut out, frame, grid, inks, place, &taken);
    out.push_str("Q\n");
    // At the weight of a mark and not of a cut. The square is the one shape on
    // the sheet the legend tells a person to measure and not to cut, so a
    // sheet carrying three pieces would otherwise show four heavy closed
    // outlines and mean three — which is the distinction `DRAWN` exists for.
    let _ = writeln!(out, "{} w", number(points(DRAWN)));
    legend::square(&mut out);
    join::marks(&mut out, frame, place);
    legend::says(&mut out, frame, inks, grid.size(), place);
    (out, unsaid)
}

/// What every piece on the sheet says about itself, and how many of them found
/// nowhere to say it.
///
/// Each piece is told the outlines of the others so that its own words never
/// end up inside one of them, where the paper would no longer say whose they
/// are.
///
/// And each block it places joins the list the next one keeps off, in the same
/// shape the node names arrive in. Two pieces of one sheet whose boxes open at
/// the same height want the same corner of it, and a list that only ever held
/// the names would let the second one print over the first.
fn words(
    out: &mut String,
    frame: &Frame,
    grid: &Grid,
    inks: &[Ink],
    place: &Place,
    named: &[[f64; 4]],
) -> usize {
    let mut taken = named.to_vec();
    let mut unsaid = 0;
    for (rank, (cloth, ink)) in grid.cloths().iter().zip(inks).enumerate() {
        let others: Vec<&[[f64; 2]]> = grid
            .cloths()
            .iter()
            .enumerate()
            .filter(|&(other, _)| other != rank)
            .map(|(_, cloth)| cloth.cut())
            .collect();
        let over = Over {
            taken: &taken,
            others: &others,
        };
        match ink::says(out, frame, cloth, ink, place, &over) {
            Some(held) => taken.extend(held),
            None => unsaid += 1,
        }
    }
    unsaid
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
