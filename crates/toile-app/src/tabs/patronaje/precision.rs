use eframe::egui::{Align2, FontId, Painter, Rect, Stroke, StrokeKind, pos2, vec2};
use toile_engine::draft::Binding;

use super::gesture::{self, Drag};
use super::view::View;
use crate::theme::Theme;
use crate::widgets::CORNER;

/// The size of the box, in screen points, before its text asks for more.
const BOX: [f32; 2] = [124.0, 26.0];

/// Where the box sits from the node it is writing, in screen points.
const OFF: [f32; 2] = [16.0, -34.0];

const HINT: &str = "Enter · Esc";

/// The box where an exact measurement is typed in the middle of a drag.
///
/// It is painted on the mat rather than opened as a window: the keys of a
/// gesture stay routed in one place, and nothing competes for the focus.
/// It reads whatever the formula field reads, so `22` and `cintura / 4 + 1`
/// are both answers to the same question.
pub fn show(p: &Painter, theme: &Theme, view: View, drag: &Drag) {
    let Some(typed) = drag.typed.as_ref() else {
        return;
    };
    let font = FontId::monospace(12.0);
    let text = format!("{}|", typed.buffer);
    let written = p.layout_no_wrap(text.clone(), font.clone(), theme.ink);
    let anchor = view.to_screen(drag.to) + vec2(OFF[0], OFF[1]);
    let width = BOX[0].max(written.size().x + 74.0);
    let rect = Rect::from_min_size(anchor, vec2(width, BOX[1]));
    let edge = if fault(&typed.buffer) {
        theme.alert
    } else {
        theme.accent
    };
    p.rect(
        rect,
        CORNER,
        theme.raised,
        Stroke::new(1.0, edge),
        StrokeKind::Inside,
    );
    p.text(
        rect.left_center() + vec2(9.0, 0.0),
        Align2::LEFT_CENTER,
        gesture::name(typed.axis),
        FontId::proportional(11.0),
        theme.muted,
    );
    p.text(
        pos2(rect.left() + 24.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        font,
        theme.ink,
    );
    p.text(
        rect.right_center() - vec2(8.0, 0.0),
        Align2::RIGHT_CENTER,
        HINT,
        FontId::monospace(9.0),
        theme.muted,
    );
}

/// Whether what has been typed so far is something the parser refuses.
///
/// An empty box is not a fault: nothing has been said yet.
fn fault(buffer: &str) -> bool {
    let written = buffer.trim();
    !written.is_empty() && Binding::parse(written).is_err()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_box_is_not_a_fault_yet() {
        assert!(!fault(""));
        assert!(!fault("   "));
    }

    #[test]
    fn the_box_reads_what_the_formula_field_reads() {
        assert!(!fault("22"));
        assert!(!fault(" cintura / 4 + 1 "));
        assert!(fault("cintura /"));
        assert!(fault("22 cm"));
    }
}
