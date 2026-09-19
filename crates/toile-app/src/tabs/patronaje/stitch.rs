use eframe::egui::{Align2, FontId, Painter, Pos2, Shape, Stroke, Vec2};
use toile_engine::draft::{Doc, EdgeRange, Seam, SeamKey, SeamOrientation};

use super::pick::away;
use super::sew::{self, Pick, Spread};
use super::tract::Tract;
use super::view::View;
use crate::theme::Theme;

/// How wide the thread along a sewn stretch is drawn, chosen and not.
const THREAD: [f32; 2] = [5.0, 3.5];

/// How much of the seam colour a seam nobody chose is drawn in.
const FAINT: f32 = 0.5;

/// The room between two arrowheads along a side, in screen points.
const PITCH: f32 = 64.0;

/// How far off its side a seam's number sits, in screen points.
const OFFSET: f32 = 13.0;

/// Every seam of the product, drawn on the two stretches it joins.
///
/// Both sides wear the same thread and the same number, which is the number
/// the inspector lists the seam under, so the two halves read as one seam
/// however far apart the pieces lie. The arrowheads are the pairing itself:
/// each side is drawn in the direction it is walked while the two are sewn,
/// and the dot is where that walk begins, so a seam that would sew the pieces
/// twisted shows its two sides running against each other on the mat. The
/// chosen seam also draws the two reaches between the ends that meet, and
/// reaches that cross are a twist seen before it is draped.
pub fn seams(
    p: &Painter,
    theme: &Theme,
    doc: &Doc,
    spread: &[Spread],
    (view, chosen): (View, Option<SeamKey>),
) {
    for (index, (key, seam)) in doc.seams.iter().enumerate() {
        let Some([a, b]) = sides(spread, seam) else {
            continue;
        };
        let lit = chosen == Some(key);
        let [a, b] = [a, b].map(|side| on_glass(&side, view));
        for side in [&a, &b] {
            thread(p, theme, side, lit);
            number(p, theme, side, index + 1, lit);
        }
        if lit {
            let reach = Stroke::new(1.0, theme.seam.gamma_multiply(FAINT));
            let ends = [a.first().zip(b.first()), a.last().zip(b.last())];
            for (&from, &to) in ends.into_iter().flatten() {
                p.line_segment([from, to], reach);
            }
        }
    }
}

/// The side already picked and the tract a press would pick next, while the
/// sewing tool is in hand.
pub fn picking(
    p: &Painter,
    theme: &Theme,
    spread: &[Spread],
    view: View,
    (first, over): (Option<Pick>, Option<Pick>),
) {
    if let Some(tract) = over
        .filter(|&it| Some(it) != first)
        .and_then(|it| sew::tract_of(spread, it))
    {
        let ink = Stroke::new(THREAD[0], theme.seam.gamma_multiply(FAINT));
        p.add(Shape::line(on_glass(&tract.line, view), ink));
    }
    if let Some(tract) = first.and_then(|it| sew::tract_of(spread, it)) {
        thread(p, theme, &on_glass(&tract.line, view), true);
    }
}

/// The two sides of a seam as lines on the mat, each in the direction the
/// pairing walks it: side A head to tail, side B the same way when the seam
/// is aligned and tail to head when it is opposed.
pub fn sides(spread: &[Spread], seam: &Seam) -> Option<[Vec<[f64; 2]>; 2]> {
    let of = |range: EdgeRange| {
        let piece = range.piece()?;
        let on = spread.iter().find(|it| it.piece == piece)?;
        let line = stretch(&on.tracts, range);
        (line.len() >= 2).then_some(line)
    };
    let (a, mut b) = (of(seam.a)?, of(seam.b)?);
    if seam.orientation == SeamOrientation::Opposed {
        b.reverse();
    }
    Some([a, b])
}

/// The line a stretch of contour draws, from its head anchor to its tail.
///
/// The anchors' fractions are honoured, so a seam that starts part way along a
/// tract is drawn from there and not from the node behind it. A tail behind
/// its head on the same tract goes the long way round, as the contour runs. A
/// stretch whose two ends are one place draws nothing: the engine pairs
/// nothing along it, and a thread round the whole piece would say it did.
fn stretch(tracts: &[Tract], at: EdgeRange) -> Vec<[f64; 2]> {
    let Some(start) = tracts.iter().position(|it| it.node == at.head.from) else {
        return Vec::new();
    };
    if at.head.from == at.tail.from {
        if at.tail.t.to_bits() == at.head.t.to_bits() {
            return Vec::new();
        }
        if at.tail.t > at.head.t {
            return cut(&tracts[start].line, at.head.t, at.tail.t);
        }
    }
    let mut out = cut(&tracts[start].line, at.head.t, 1.0);
    for step in 1..=tracts.len() {
        let tract = &tracts[(start + step) % tracts.len()];
        if tract.node == at.tail.from {
            if at.tail.t > 0.0 {
                out.extend(cut(&tract.line, 0.0, at.tail.t));
            }
            return out;
        }
        out.extend(tract.line.iter().copied());
    }
    Vec::new()
}

/// The part of a line between two fractions of its own length.
fn cut(line: &[[f64; 2]], from: f64, to: f64) -> Vec<[f64; 2]> {
    let total: f64 = line.windows(2).map(|pair| away(pair[0], pair[1])).sum();
    if total <= f64::EPSILON || (from <= 0.0 && to >= 1.0) {
        return line.to_vec();
    }
    let (lo, hi) = (from.clamp(0.0, 1.0) * total, to.clamp(0.0, 1.0) * total);
    let mut out = Vec::new();
    let mut along = 0.0;
    for pair in line.windows(2) {
        let span = away(pair[0], pair[1]);
        let at = |arc: f64| {
            let t = if span > 0.0 {
                (arc - along) / span
            } else {
                0.0
            };
            [0, 1].map(|k| pair[0][k] + (pair[1][k] - pair[0][k]) * t)
        };
        if lo >= along && lo <= along + span {
            out.push(at(lo));
        }
        if along + span > lo && along + span < hi {
            out.push(pair[1]);
        }
        if hi >= along && hi <= along + span {
            out.push(at(hi));
            break;
        }
        along += span;
    }
    out
}

fn on_glass(line: &[[f64; 2]], view: View) -> Vec<Pos2> {
    line.iter().map(|&at| view.to_screen(at)).collect()
}

/// One side: the thread along it, the dot the pairing starts from, and the
/// arrowheads that say which way it goes from there.
fn thread(p: &Painter, theme: &Theme, side: &[Pos2], lit: bool) {
    let (width, ink) = if lit {
        (THREAD[0], theme.seam)
    } else {
        (THREAD[1], theme.seam.gamma_multiply(FAINT))
    };
    p.add(Shape::line(side.to_vec(), Stroke::new(width, ink)));
    if let Some(&start) = side.first() {
        p.circle_filled(start, width, theme.seam);
    }
    let length: f32 = side
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).length())
        .sum();
    let heads = ((length / PITCH) as usize).max(1);
    for head in 0..heads {
        let arc = length * (head as f32 + 0.5) / heads as f32;
        if let Some((tip, run)) = place(side, arc) {
            let across = run.rot90() * 4.5;
            let wings = [tip - run * 9.0 + across, tip, tip - run * 9.0 - across];
            p.add(Shape::line(wings.to_vec(), Stroke::new(1.6, theme.ink)));
        }
    }
}

/// The seam's number beside the middle of a side, on a disc of the mat's own
/// ground so it reads over paper, grid and outline alike.
fn number(p: &Painter, theme: &Theme, side: &[Pos2], ordinal: usize, lit: bool) {
    let length: f32 = side
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).length())
        .sum();
    let Some((middle, run)) = place(side, length / 2.0) else {
        return;
    };
    let at = middle + run.rot90() * OFFSET;
    let ink = if lit { theme.seam } else { theme.ink_soft };
    p.circle(at, 8.0, theme.mat, Stroke::new(1.0, theme.seam));
    let font = FontId::monospace(10.0);
    p.text(at, Align2::CENTER_CENTER, ordinal.to_string(), font, ink);
}

/// The place `arc` screen points along a line, and the way the line runs
/// there.
fn place(line: &[Pos2], arc: f32) -> Option<(Pos2, Vec2)> {
    let mut along = 0.0;
    let mut last = None;
    for pair in line.windows(2) {
        let run = pair[1] - pair[0];
        let span = run.length();
        if span <= f32::EPSILON {
            continue;
        }
        let unit = run / span;
        if arc <= along + span {
            return Some((pair[0] + unit * (arc - along), unit));
        }
        along += span;
        last = Some((pair[1], unit));
    }
    last
}

#[cfg(test)]
mod tests;
