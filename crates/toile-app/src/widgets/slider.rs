use eframe::egui::{Align2, FontId, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use super::{CORNER, PAD};
use crate::theme::Theme;

/// Height of a row: the name and the number, over the rail they belong to.
const ROW_H: f32 = 48.0;

/// Room the number keeps at the right of the name.
const VALUE_W: f32 = 72.0;

/// How thick the rail is drawn, and how far the pointer may stray from it and
/// still be on it.
const RAIL_H: f32 = 3.0;
const REACH: f32 = 9.0;

/// The knob riding the rail, and the ring it wears under the pointer.
const KNOB: f32 = 5.5;

/// One number a pointer drags along a rail.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Track<'a> {
    /// The name on the left.
    pub label: &'a str,
    /// The lowest and the highest the rail offers.
    pub span: (f64, f64),
    /// The smallest move the rail makes, which is also what it reads to.
    pub step: f64,
    /// What the document holds right now.
    pub value: f64,
    /// Written after the number, when it has one.
    pub unit: &'a str,
    /// Decimals the number is written to.
    pub decimals: usize,
}

/// A number dragged along a rail, answering the value the pointer asks for.
///
/// The answer is read off where the pointer is and nowhere else, so a pointer
/// held still asks for the very number it asked for last frame and the caller
/// writes nothing. `None` while nobody has hold of it.
///
/// Nothing is typed here. The box that takes a formula belongs to `field.rs`,
/// and it keeps a buffer because half a formula is not a number; a rail has no
/// half-written state to keep, which is what lets it answer in one call.
pub fn track(ui: &mut Ui, theme: &Theme, id: Id, row: &Track<'_>) -> (Response, Option<f64>) {
    let (_, rect) = ui.allocate_space(vec2(ui.available_width(), ROW_H));
    let top = rect.top() + 32.0;
    let rail = Rect::from_min_max(
        pos2(rect.left() + PAD + KNOB, top),
        pos2(rect.right() - PAD - KNOB, top + RAIL_H),
    );
    let resp = ui.interact(rail.expand2(vec2(KNOB, REACH)), id, Sense::click_and_drag());
    let asked = resp
        .interact_pointer_pos()
        .map(|at| quantised(row, rail, at.x));
    paint(ui, theme, (rect, rail), row, (asked, resp.hovered()));
    (resp, asked)
}

/// The value the rail reads at `x`, snapped to the row's step and kept inside
/// its span.
///
/// Snapped so that a rail held still reports one number rather than a new one
/// per pixel of jitter, and so that the number written is the number drawn.
fn quantised(row: &Track<'_>, rail: Rect, x: f32) -> f64 {
    let (lo, hi) = row.span;
    let along = f64::from((x - rail.left()) / rail.width().max(1.0)).clamp(0.0, 1.0);
    let raw = lo + along * (hi - lo);
    ((raw / row.step).round() * row.step).clamp(lo, hi)
}

/// How far along the rail a value sits, from 0 at its low end to 1 at its
/// high one.
fn fraction(row: &Track<'_>, value: f64) -> f32 {
    let (lo, hi) = row.span;
    let span = hi - lo;
    if span.abs() < f64::EPSILON {
        return 0.0;
    }
    ((value - lo) / span).clamp(0.0, 1.0) as f32
}

/// The row as it looks this frame.
///
/// What the pointer is asking for is drawn rather than what the document
/// holds, so the knob follows the hand on the frame it moves instead of the
/// one after the write comes back.
fn paint(
    ui: &Ui,
    theme: &Theme,
    (rect, rail): (Rect, Rect),
    row: &Track<'_>,
    (asked, lit): (Option<f64>, bool),
) {
    let value = asked.unwrap_or(row.value);
    let p = ui.painter();
    p.text(
        pos2(rect.left() + PAD, rect.top() + 13.0),
        Align2::LEFT_CENTER,
        row.label,
        FontId::proportional(12.0),
        theme.ink_soft,
    );
    let boxed = Rect::from_min_max(
        pos2(rect.right() - PAD - VALUE_W, rect.top() + 2.0),
        pos2(rect.right() - PAD, rect.top() + 24.0),
    );
    p.rect(
        boxed,
        CORNER,
        theme.raised,
        Stroke::new(1.0, theme.line),
        StrokeKind::Inside,
    );
    p.text(
        boxed.right_center() - vec2(8.0, 0.0),
        Align2::RIGHT_CENTER,
        written(row, value),
        FontId::monospace(12.0),
        theme.ink,
    );
    let knob = pos2(
        rail.left() + rail.width() * fraction(row, value),
        rail.center().y,
    );
    p.rect_filled(rail, CORNER, theme.line);
    p.rect_filled(
        Rect::from_min_max(rail.left_top(), pos2(knob.x, rail.bottom())),
        CORNER,
        theme.accent,
    );
    p.circle_filled(knob, KNOB, theme.accent);
    if lit {
        p.circle_stroke(knob, KNOB + 3.0, Stroke::new(1.0, theme.accent));
    }
}

/// The number as the box writes it, with its unit after it when it has one.
fn written(row: &Track<'_>, value: f64) -> String {
    if row.unit.is_empty() {
        return format!("{:.*}", row.decimals, value);
    }
    format!("{:.*} {}", row.decimals, value, row.unit)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "a rail snapped to its step reads back the very number it snapped to"
    )]

    use super::*;

    fn row() -> Track<'static> {
        Track {
            label: "razón",
            span: (20.0, 200.0),
            step: 1.0,
            value: 100.0,
            unit: "%",
            decimals: 0,
        }
    }

    fn rail() -> Rect {
        Rect::from_min_max(pos2(100.0, 0.0), pos2(280.0, 3.0))
    }

    #[test]
    fn a_pointer_past_either_end_asks_for_that_end_and_no_further() {
        assert_eq!(quantised(&row(), rail(), -400.0), 20.0);
        assert_eq!(quantised(&row(), rail(), 4000.0), 200.0);
    }

    #[test]
    fn the_middle_of_the_rail_is_the_middle_of_the_span() {
        assert_eq!(quantised(&row(), rail(), 190.0), 110.0);
        assert_eq!(fraction(&row(), 110.0), 0.5);
    }

    /// Two pointer positions inside one step ask for one number, which is what
    /// keeps a rail held still from writing on every frame.
    #[test]
    fn a_pointer_that_has_not_left_its_step_asks_for_the_same_number() {
        let (row, rail) = (row(), rail());
        assert_eq!(quantised(&row, rail, 190.0), quantised(&row, rail, 190.4));
    }

    /// A strength rail reads in tenths, and the tenth it reads is the one the
    /// box writes: a value the document already holds comes back bit for bit,
    /// so a press that moves nothing asks for no edit.
    #[test]
    fn a_tenth_comes_back_as_the_very_number_the_document_holds() {
        let row = Track {
            label: "rigidez",
            span: (0.1, 50.0),
            step: 0.1,
            value: 10.0,
            unit: "×",
            decimals: 1,
        };
        let rail = rail();
        let at = rail.left() + rail.width() * fraction(&row, 10.0);
        assert_eq!(quantised(&row, rail, at).to_bits(), 10.0_f64.to_bits());
    }

    #[test]
    fn a_row_with_no_unit_writes_the_bare_number() {
        let bare = Track { unit: "", ..row() };
        assert_eq!(written(&bare, 85.0), "85");
        assert_eq!(written(&row(), 85.0), "85 %");
    }
}
