use eframe::egui::{Align2, FontId, Painter, Rect, Stroke, pos2};
use toile_engine::draft::Axis;

use super::view::View;
use crate::theme::Theme;

/// Width of the two ruler bands, in screen points.
pub const BAND: f32 = 20.0;

/// Screen points a numbered tick asks for, which is what picks the decade.
const TICK_GAP: f64 = 72.0;

/// How far a numbered tick reaches into its band.
const MAJOR: f32 = 13.0;
const MINOR: f32 = 6.0;

/// The distance between numbered ticks, in centimetres, on the 1-2-5 ladder.
pub fn step_cm(scale: f64) -> f64 {
    let raw = TICK_GAP / scale.max(1.0e-6);
    let decade = 10.0_f64.powi(raw.log10().floor() as i32);
    let mantissa = raw / decade;
    let rung = if mantissa <= 1.0 {
        1.0
    } else if mantissa <= 2.0 {
        2.0
    } else if mantissa <= 5.0 {
        5.0
    } else {
        10.0
    };
    rung * decade
}

/// The two bands, in centimetres of document: numbers across the top, numbers
/// down the left, both following whatever the view is showing.
pub fn show(p: &Painter, theme: &Theme, rect: Rect, view: View) {
    let top = Rect::from_min_max(rect.left_top(), pos2(rect.right(), rect.top() + BAND));
    let side = Rect::from_min_max(rect.left_top(), pos2(rect.left() + BAND, rect.bottom()));
    for band in [top, side] {
        p.rect_filled(band, 0.0, theme.panel);
    }
    let edge = Stroke::new(1.0, theme.line);
    p.line_segment([top.left_bottom(), top.right_bottom()], edge);
    p.line_segment([side.right_top(), side.right_bottom()], edge);

    let step = step_cm(view.scale());
    let (near, far) = (
        view.to_document(rect.left_top()),
        view.to_document(rect.right_bottom()),
    );
    let half = (step * view.scale() / 2.0) as f32;
    for (axis, near, far) in [(Axis::X, near[0], far[0]), (Axis::Y, near[1], far[1])] {
        for k in (near / step).floor() as i64..=(far / step).ceil() as i64 {
            let cm = k as f64 * step;
            let along = match axis {
                Axis::X => view.to_screen([cm, 0.0]).x,
                Axis::Y => view.to_screen([0.0, cm]).y,
            };
            mark(p, theme, rect, axis, along, &label(cm, step));
            mark(p, theme, rect, axis, along + half, "");
        }
    }
}

/// One tick in a band, numbered when it is given a label.
fn mark(p: &Painter, theme: &Theme, rect: Rect, axis: Axis, along: f32, text: &str) {
    let (head, tail) = match axis {
        Axis::X => (rect.left() + BAND, rect.right()),
        Axis::Y => (rect.top() + BAND, rect.bottom()),
    };
    if along < head || along > tail {
        return;
    }
    let reach = if text.is_empty() { MINOR } else { MAJOR };
    let (a, b) = match axis {
        Axis::X => (
            pos2(along, rect.top() + BAND - reach),
            pos2(along, rect.top() + BAND),
        ),
        Axis::Y => (
            pos2(rect.left() + BAND - reach, along),
            pos2(rect.left() + BAND, along),
        ),
    };
    p.line_segment([a, b], Stroke::new(1.0, theme.muted));
    if text.is_empty() {
        return;
    }
    let font = FontId::monospace(9.0);
    let at = match axis {
        Axis::X => pos2(along + 3.0, rect.top() + 1.0),
        Axis::Y => pos2(rect.left() + 2.0, along + 2.0),
    };
    p.text(at, Align2::LEFT_TOP, text, font, theme.muted);
}

/// The number on a tick: as coarse as the step it belongs to.
fn label(cm: f64, step: f64) -> String {
    if step < 1.0 {
        format!("{cm:.1}")
    } else {
        format!("{cm:.0}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mantissa of a step, once its decade is divided out.
    fn rung(step: f64) -> f64 {
        let decade = 10.0_f64.powi(step.log10().floor() as i32);
        (step / decade * 1.0e6).round() / 1.0e6
    }

    #[test]
    fn tick_step_follows_a_one_two_five_decade() {
        for scale in [0.4, 1.0, 3.7, 12.0, 37.8, 96.0, 240.0] {
            let step = step_cm(scale);
            assert!(
                [1.0, 2.0, 5.0].contains(&rung(step)),
                "scale {scale} gave step {step}"
            );
        }
    }

    #[test]
    fn a_closer_view_asks_for_a_finer_step() {
        let mut last = f64::INFINITY;
        for scale in [0.4, 4.0, 40.0, 240.0] {
            let step = step_cm(scale);
            assert!(step < last, "scale {scale} gave step {step}");
            last = step;
        }
    }

    #[test]
    fn a_numbered_tick_lands_near_the_gap_it_asked_for() {
        for scale in [0.4, 1.0, 3.7, 12.0, 37.8, 96.0, 240.0] {
            let gap = step_cm(scale) * scale;
            assert!((TICK_GAP / 5.0..=TICK_GAP * 2.0).contains(&gap), "{gap}");
        }
    }
}
