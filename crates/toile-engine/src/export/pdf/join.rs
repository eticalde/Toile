use std::fmt::Write;

use super::super::units::{CAPTION, DRAWN, MARGIN, number};
use super::grid::OVERLAP;
use super::paper::points;
use super::place::Place;
use super::sheet::Frame;
use super::text::show;

/// Half a registration cross, in millimetres.
///
/// Eight millimetres across, in a band twenty wide: the cross has six clear
/// millimetres either side of it, which is what lets a hand that cut a
/// millimetre wide still find the whole mark on both sheets.
const ARM: f64 = 4.0;

/// The dashes the line to cut on is drawn with, in points.
const DASH: &str = "2 2";

/// Where along a shared band the crosses go, as shares of the sheet's own cell.
///
/// Three and not one, and spread rather than bunched: two marks half a sheet
/// apart fix the turn of the paper as well as its place, and the third is what
/// tells a person which way it went wrong.
const CROSSES: [f64; 3] = [0.25, 0.5, 0.75];

/// What a person needs to put this sheet against its neighbours: the line to
/// cut on, and the marks that fall on the band both sheets carry.
///
/// Nothing is drawn on an edge with no neighbour. The outer edge of a piece is
/// where its own cut line runs, and a dashed line inviting a person to cut
/// there would be an invitation to cut the pattern away.
pub(super) fn marks(out: &mut String, frame: &Frame, place: &Place) {
    let [wide, tall] = frame.cell();
    let joins = place.joins;
    // At the weight of everything that is not the cut line, because the one
    // line on this sheet that must never be mistaken for the cut line is the
    // line that is not the piece.
    let _ = writeln!(out, "{} w", number(points(DRAWN)));
    let _ = writeln!(out, "q [{DASH}] 0 d");
    if joins.left.is_some() {
        upright(out, frame, MARGIN, tall);
    }
    if joins.right.is_some() {
        upright(out, frame, MARGIN + wide, tall);
    }
    if joins.above.is_some() {
        flat(out, frame, MARGIN, wide);
    }
    if joins.below.is_some() {
        flat(out, frame, MARGIN + tall, wide);
    }
    out.push_str("Q\n");
    // Named by the two sheets, which is the only name both of them arrive at:
    // an index of the grid is worn by every sheet of its column or its row, so
    // on a piece of two columns and six rows a dozen sheets would all say `C1`
    // and a person holding two of them could tape the wrong pair together.
    let named = |low: usize, high: usize| (low.min(high), low.max(high));
    if let Some(beside) = joins.left {
        let (low, high) = named(beside, place.number);
        down_the_band(
            out,
            frame,
            MARGIN + OVERLAP / 2.0,
            &format!("C{low}-{high}"),
        );
    }
    if let Some(beside) = joins.right {
        let (low, high) = named(place.number, beside);
        down_the_band(
            out,
            frame,
            MARGIN + wide - OVERLAP / 2.0,
            &format!("C{low}-{high}"),
        );
    }
    if let Some(beside) = joins.above {
        let (low, high) = named(beside, place.number);
        along_the_band(
            out,
            frame,
            MARGIN + OVERLAP / 2.0,
            &format!("F{low}-{high}"),
        );
    }
    if let Some(beside) = joins.below {
        let (low, high) = named(place.number, beside);
        along_the_band(
            out,
            frame,
            MARGIN + tall - OVERLAP / 2.0,
            &format!("F{low}-{high}"),
        );
    }
}

/// The line to cut on down one side of the cell.
fn upright(out: &mut String, frame: &Frame, across: f64, tall: f64) {
    let mut path = String::new();
    segment(
        &mut path,
        frame.page(across, MARGIN),
        frame.page(across, MARGIN + tall),
    );
    let _ = writeln!(out, "{path}S");
}

/// The line to cut on along the top or the bottom of the cell.
fn flat(out: &mut String, frame: &Frame, down: f64, wide: f64) {
    let mut path = String::new();
    segment(
        &mut path,
        frame.page(MARGIN, down),
        frame.page(MARGIN + wide, down),
    );
    let _ = writeln!(out, "{path}S");
}

/// The crosses down the middle of a band this sheet shares with the one beside
/// it.
///
/// `across` is the middle of that band, which is the same place in the document
/// on both sheets: that is the whole of why matching the two marks matches the
/// two drawings, and why the mark is placed from the band and never from the
/// piece.
fn down_the_band(out: &mut String, frame: &Frame, across: f64, joint: &str) {
    let tall = frame.cell()[1];
    for share in CROSSES {
        cross(out, frame, [across, MARGIN + tall * share], joint);
    }
}

/// The crosses along the middle of a band this sheet shares with the one above
/// or below it.
fn along_the_band(out: &mut String, frame: &Frame, down: f64, joint: &str) {
    let wide = frame.cell()[0];
    for share in CROSSES {
        cross(out, frame, [MARGIN + wide * share, down], joint);
    }
}

/// One registration cross, named after the two sheets the joint holds together,
/// lower number first, so that both of them print the same name for it.
fn cross(out: &mut String, frame: &Frame, [across, down]: [f64; 2], joint: &str) {
    let mut path = String::new();
    segment(
        &mut path,
        frame.page(across - ARM, down),
        frame.page(across + ARM, down),
    );
    segment(
        &mut path,
        frame.page(across, down - ARM),
        frame.page(across, down + ARM),
    );
    let _ = writeln!(out, "{path}S");
    show(
        out,
        frame.page(across + 1.0, down - ARM - 1.0),
        CAPTION,
        joint,
    );
}

/// One straight run of a path, in points.
fn segment(path: &mut String, from: [f64; 2], to: [f64; 2]) {
    let _ = write!(
        path,
        "{} {} m {} {} l ",
        number(from[0]),
        number(from[1]),
        number(to[0]),
        number(to[1])
    );
}
