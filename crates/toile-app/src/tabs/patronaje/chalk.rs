use eframe::egui::{Color32, Painter, Pos2, Shape, Stroke, Vec2};
use toile_engine::draft::{LineKey, LineKind, NotchCount, PointKey};

use super::inner::{Drawn, Tick};
use super::trace::Place;
use super::view::View;
use crate::theme::Theme;

/// How long the bar across each end of a finished hole is, in screen points.
const BAR: f32 = 8.0;

/// How long a notch's cut is drawn, in screen points.
const CUT: f32 = 10.0;

/// And how wide, which with the ink of a cut line is what a mark is drawn as.
pub(super) const NOTCH_W: f32 = 1.4;

/// The room between two cuts of a double or triple notch.
const PITCH: f32 = 3.0;

/// How much of its ink a line nobody chose is drawn in.
pub(super) const FAINT: f32 = 0.75;

/// How much wider the line chosen is drawn.
const LIT: f32 = 0.7;

/// How big the dot on a place of the chosen line is, in screen points.
const DOT: f32 = 3.0;

/// How a kind of internal line is stroked.
///
/// Meaning first and stroke second: the document keeps what a line is *for*,
/// and this is the one place that becomes something to look at. No two kinds
/// are stroked alike, so a fold is told from a topstitch without anybody
/// reading the panel. A pocket mouth and a buttonhole are the one pair drawn
/// in the same ink and pattern — both are cut open — and the bar across each
/// end is what tells them apart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chalk {
    /// The ink it is drawn in.
    pub ink: Color32,
    /// How wide, in screen points.
    pub width: f32,
    /// The dash and the gap after it; continuous when there is none.
    pub dash: Option<(f32, f32)>,
    /// Whether each end wears the bar of a finished hole.
    pub bars: bool,
}

/// How a kind is stroked, out of the theme's own roles.
///
/// The two kinds the cutter opens wear the ink a piece's cut line wears,
/// because that is what they are: a place the scissors or the machine goes,
/// which is why they are the two continuous ones. A fold is the drawing's own
/// axis, so it takes the ink of a guide. A topstitch is thread, so it takes the
/// ink of a seam. A placement is a label on the cloth — "the flap goes here" —
/// and a reference carries nothing at all, so it is written in the ink of
/// something that is not there to be used.
pub fn chalk(kind: LineKind, theme: &Theme) -> Chalk {
    let (ink, width, dash, bars) = match kind {
        LineKind::Slit => (theme.outline, 1.8, None, false),
        LineKind::Buttonhole => (theme.outline, 1.8, None, true),
        LineKind::Fold => (theme.measure, 1.4, Some((9.0, 4.0)), false),
        LineKind::Stitch => (theme.seam, 1.4, Some((3.0, 3.0)), false),
        LineKind::Placement => (theme.ink_soft, 1.4, Some((5.0, 5.0)), false),
        LineKind::Reference => (theme.muted, 1.0, Some((1.5, 4.0)), false),
    };
    Chalk {
        ink,
        width,
        dash,
        bars,
    }
}

/// Every internal line of a piece, drawn over its paper.
///
/// Over the paper and over the cut line, because that is where the marks sit: a
/// fold or a pocket mouth is drawn on the piece, not under it. One that
/// resolves nowhere draws nothing, and the panel says which one it was.
pub fn lines(p: &Painter, theme: &Theme, drawn: &[Drawn], (view, chosen): (View, Option<LineKey>)) {
    for it in drawn {
        if it.run.len() < 2 {
            continue;
        }
        let lit = chosen == Some(it.line);
        let mut ink = chalk(it.kind, theme);
        if lit {
            ink.width += LIT;
        } else {
            ink.ink = ink.ink.gamma_multiply(FAINT);
        }
        let run: Vec<Pos2> = it.run.iter().map(|&at| view.to_screen(at)).collect();
        stroke(p, ink, &run);
        if ink.bars {
            ends(p, ink, &run);
        }
        if lit {
            places(p, ink, view, &it.loose);
        }
    }
}

/// The places of the chosen line that a hand can take hold of.
///
/// Only the chosen line's, because only those answer a press, and in the line's
/// own ink rather than a node's, so a mark on the cloth is never read as a
/// corner of the piece. A place anchored to the contour wears none: it is a
/// fraction of a tract and moving it is a question about the contour.
fn places(p: &Painter, chalk: Chalk, view: View, loose: &[(PointKey, [f64; 2])]) {
    for &(_, at) in loose {
        p.circle_filled(view.to_screen(at), DOT, chalk.ink);
    }
}

/// Every notch of a piece, cut across the contour where it sits.
///
/// Across and not into the cloth: which side of the line the seam allowance
/// lies on is not something the contour says here, and a tick that crosses it
/// is right either way.
pub fn notches(p: &Painter, theme: &Theme, ticks: &[Tick], view: View) {
    let ink = Stroke::new(NOTCH_W, theme.outline);
    for tick in ticks {
        let at = view.to_screen(tick.at);
        let along = Vec2::new(tick.along[0] as f32, tick.along[1] as f32);
        if along.length() < f32::EPSILON {
            continue;
        }
        let along = along.normalized();
        let across = along.rot90() * (CUT / 2.0);
        let cuts = match tick.count {
            NotchCount::Single => 1,
            NotchCount::Double => 2,
            NotchCount::Triple => 3,
        };
        for cut in 0..cuts {
            let step = (cut as f32 - (cuts - 1) as f32 / 2.0) * PITCH;
            let middle = at + along * step;
            p.line_segment([middle - across, middle + across], ink);
        }
    }
}

/// The line being traced: the places pressed, the run through them, and the
/// rubber line to wherever the pointer is.
///
/// In the ink of a gesture and not of a kind, because until it is finished it
/// is not a line of the document and nothing has been said about what it is
/// for.
pub fn tracing(p: &Painter, theme: &Theme, view: View, pending: &[Place], rubber: [f64; 2]) {
    let pts: Vec<Pos2> = pending.iter().map(|it| view.to_screen(it.cm)).collect();
    if let Some(&last) = pts.last() {
        let guide = Stroke::new(1.0, theme.accent.gamma_multiply(0.55));
        p.line_segment([last, view.to_screen(rubber)], guide);
    }
    if pts.len() >= 2 {
        p.add(Shape::line(pts.clone(), Stroke::new(1.5, theme.accent)));
    }
    for &at in &pts {
        p.circle_filled(at, 3.0, theme.accent);
    }
}

/// One run, continuous or dashed as its kind asks.
fn stroke(p: &Painter, chalk: Chalk, run: &[Pos2]) {
    let ink = Stroke::new(chalk.width, chalk.ink);
    match chalk.dash {
        None => {
            p.add(Shape::line(run.to_vec(), ink));
        }
        Some((dash, gap)) => p.extend(Shape::dashed_line(run, ink, dash, gap)),
    }
}

/// The bar across each end of a finished hole, which is what tells a buttonhole
/// from the pocket mouth it is drawn like.
fn ends(p: &Painter, chalk: Chalk, run: &[Pos2]) {
    let ink = Stroke::new(chalk.width, chalk.ink);
    let inward = run.len().checked_sub(2).and_then(|k| run.get(k));
    for (end, next) in [(run.first(), run.get(1)), (run.last(), inward)] {
        let (Some(&end), Some(&next)) = (end, next) else {
            continue;
        };
        let out = end - next;
        if out.length() < f32::EPSILON {
            continue;
        }
        let across = out.normalized().rot90() * (BAR / 2.0);
        p.line_segment([end - across, end + across], ink);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The six kinds, in the order the document declares them.
    const KINDS: [LineKind; 6] = [
        LineKind::Fold,
        LineKind::Stitch,
        LineKind::Slit,
        LineKind::Buttonhole,
        LineKind::Placement,
        LineKind::Reference,
    ];

    /// No two kinds are drawn the same.
    ///
    /// The stroke is the whole of what a person reads off the mat: two kinds
    /// that shared an ink, a width, a pattern and their end bars would be one
    /// mark with two meanings, and the pattern would say less on paper than the
    /// file says.
    #[test]
    fn every_kind_of_line_is_stroked_unlike_every_other() {
        let theme = Theme::sastreria();
        let drawn: Vec<Chalk> = KINDS.iter().map(|&kind| chalk(kind, &theme)).collect();
        for (index, one) in drawn.iter().enumerate() {
            for other in &drawn[index + 1..] {
                assert_ne!(one, other, "two kinds stroked alike");
            }
        }
    }

    /// The two kinds the cutter opens are the two continuous ones, and the only
    /// two in the ink of a cut line: what the scissors follow is not a guide.
    #[test]
    fn the_kinds_that_open_the_cloth_are_the_ones_drawn_as_a_cut() {
        let theme = Theme::sastreria();
        for kind in KINDS {
            let drawn = chalk(kind, &theme);
            assert_eq!(
                drawn.dash.is_none(),
                kind.opens_the_cloth(),
                "{kind:?} is stroked against what the cutter does with it"
            );
            assert_eq!(
                drawn.ink == theme.outline,
                kind.opens_the_cloth(),
                "{kind:?}"
            );
        }
        assert!(chalk(LineKind::Buttonhole, &theme).bars, "a finished hole");
        assert!(!chalk(LineKind::Slit, &theme).bars, "a mouth someone faces");
    }
}
