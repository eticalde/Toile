use eframe::egui::{Align2, FontId, Painter, Pos2, Shape, Stroke, Vec2};
use toile_engine::draft::SeamKey;

use super::sew::{self, Pick, Spread};
use super::thread::Thread;
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
/// however far apart the pieces lie, and a side of several tracts is one line
/// and so one thread. The arrowheads are the pairing itself:
/// each side is drawn in the direction it is walked while the two are sewn,
/// and the dot is where that walk begins, so a seam that would sew the pieces
/// twisted shows its two sides running against each other on the mat. The
/// chosen seam also draws the two reaches between the ends that meet, and
/// reaches that cross are a twist seen before it is draped.
pub fn seams(
    p: &Painter,
    theme: &Theme,
    threads: &[Thread],
    (view, chosen): (View, Option<SeamKey>),
) {
    for it in threads {
        let lit = chosen == Some(it.seam);
        let [a, b] = [0, 1].map(|side| on_glass(&it.sides[side], view));
        for side in [&a, &b] {
            thread(p, theme, side, lit);
            number(p, theme, side, it.ordinal, lit);
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
    if let Some(line) = over
        .filter(|&it| Some(it) != first)
        .and_then(|it| sew::line_of(spread, it))
    {
        let ink = Stroke::new(THREAD[0], theme.seam.gamma_multiply(FAINT));
        p.add(Shape::line(on_glass(&line, view), ink));
    }
    if let Some(line) = first.and_then(|it| sew::line_of(spread, it)) {
        thread(p, theme, &on_glass(&line, view), true);
    }
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
