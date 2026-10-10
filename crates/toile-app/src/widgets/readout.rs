use std::sync::Arc;

use eframe::egui::{
    self, Align2, Color32, FontId, Galley, Id, Painter, Pos2, Response, Sense, Shape, Stroke,
    StrokeKind, Ui, Vec2, vec2,
};

use super::CORNER;
use crate::theme::Theme;

/// Caption plus a boxed value: something the app knows, with nothing to press.
pub fn readout(ui: &mut Ui, theme: &Theme, caption: &str, value: &str, width: f32) {
    boxed(ui, theme, caption, value, width, None, None);
}

/// The caption as the box lays it out, so a row can be told how wide one is by
/// the layout that draws it rather than by a second reading of its own.
fn caption_of(ui: &Ui, theme: &Theme, caption: &str) -> Arc<Galley> {
    egui::WidgetText::from(
        egui::RichText::new(caption.to_uppercase())
            .monospace()
            .size(10.0)
            .extra_letter_spacing(1.2)
            .color(theme.muted),
    )
    .into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Body,
    )
}

/// How much room one readout takes, caption and box together, in points.
///
/// Through [`caption_of`], so what a row is told is what it is about to paint.
/// A row that worked its own arithmetic out had the captions nowhere in it: the
/// probador's register asked for 1748 pt of the 1288 the window leaves, and
/// every box's own comment about its own width was true the whole time.
pub fn readout_room(ui: &Ui, theme: &Theme, caption: &str, width: f32) -> f32 {
    caption_of(ui, theme, caption).size().x + ui.spacing().item_spacing.x + width
}

/// The same box over a value a press takes to the next one in order.
///
/// Marked with a cycle and never a chevron. A chevron promises a list to
/// choose from, this app opens no menu anywhere, and a press here does not
/// choose: it steps. The mark is what the press really does.
pub fn cycle(ui: &mut Ui, theme: &Theme, caption: &str, value: &str, width: f32) -> Response {
    boxed(ui, theme, caption, value, width, Some(cycle_mark), None)
}

/// The same step under an identity the caller names, so a press aimed from
/// outside the panel finds the box by what it steps and not by where the rows
/// above it happened to leave it.
pub fn cycle_named(
    ui: &mut Ui,
    theme: &Theme,
    id: Id,
    caption: &str,
    value: &str,
    width: f32,
) -> Response {
    boxed(ui, theme, caption, value, width, Some(cycle_mark), Some(id))
}

/// Caption, boxed value, and the mark that says how the box answers a press.
///
/// The mark is what carries the click: a box drawn without one senses nothing,
/// so a control that has nothing to do cannot be pressed and left wondering.
fn boxed(
    ui: &mut Ui,
    theme: &Theme,
    caption: &str,
    value: &str,
    width: f32,
    mark: Option<fn(&Painter, Pos2, Color32)>,
    named: Option<Id>,
) -> Response {
    ui.horizontal(|ui| {
        ui.label(caption_of(ui, theme, caption));
        let live = mark.is_some();
        let sense = if live { Sense::click() } else { Sense::hover() };
        // Taking the room and interacting with it are two steps, so a name the
        // caller gave stands in for the one the layout would have issued.
        let (auto, rect) = ui.allocate_space(vec2(width, 24.0));
        let resp = ui.interact(rect, named.unwrap_or(auto), sense);
        let p = ui.painter();
        // The border lifts under the pointer only where a press does
        // something: a box that answers nothing may not brighten as if it did.
        let edge = if live && resp.hovered() {
            theme.muted
        } else {
            theme.line
        };
        p.rect(
            rect,
            CORNER,
            theme.raised,
            Stroke::new(1.0, edge),
            StrokeKind::Inside,
        );
        // Clipped to its own box, because the row budgets by the box and a
        // value is not always the length the box was sized for: a refusal from
        // the oven reaches the body reading verbatim, and the longest of them
        // runs 372 pt past a box of 170. Cut off, a reading is plainly cut off;
        // painted over its neighbour, it reads as the neighbour's.
        p.with_clip_rect(rect).text(
            rect.left_center() + vec2(10.0, 0.0),
            Align2::LEFT_CENTER,
            value,
            FontId::proportional(12.0),
            theme.ink,
        );
        if let Some(mark) = mark {
            mark(p, rect.right_center() - vec2(12.0, 0.0), theme.muted);
        }
        resp
    })
    .inner
}

/// A ring left open, with an arrowhead at the head of the turn.
fn cycle_mark(p: &Painter, centre: Pos2, color: Color32) {
    let stroke = Stroke::new(1.2, color);
    let (from, to) = (-45.0_f32, 255.0_f32);
    let at = |deg: f32| centre + Vec2::angled(deg.to_radians()) * 4.2;
    let arc: Vec<Pos2> = (0..=14)
        .map(|i| at(from + (to - from) * i as f32 / 14.0))
        .collect();
    p.add(Shape::line(arc, stroke));
    let tip = at(to);
    let wing = |turn: f32| tip + Vec2::angled((to + 90.0 + turn).to_radians()) * 3.0;
    p.add(Shape::line(vec![wing(150.0), tip, wing(-150.0)], stroke));
}
