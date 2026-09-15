use eframe::egui::{
    Align2, CursorIcon, FontId, Id, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2,
    pos2, vec2,
};

use super::field::{Editable, Edited, edit_box, note};
use super::{CORNER, PAD};
use crate::theme::Theme;

/// Height of the line a measurement takes.
const LINE_H: f32 = 30.0;
/// The room a fault takes under a measurement's box.
const FAULT_H: f32 = 14.0;
/// Height of a named formula: its name, its box, and what it comes to.
const NAMED_H: f32 = 66.0;
const VALUE_W: f32 = 84.0;
const UNIT_W: f32 = 26.0;
/// The room around a name, the same whether a press can take it or not.
const CHIP_PAD: Vec2 = vec2(6.0, 3.0);

/// Whether a name on a panel can be pressed into a formula right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mention {
    /// Information: the name, and nothing to press.
    Inert,
    /// A press puts the name into the formula being written.
    Live,
}

/// A name a formula can read, as a panel offers it.
#[derive(Debug, Clone, Copy)]
pub struct Named<'a> {
    /// The name, spelled as a formula reads it.
    pub name: &'a str,
    /// The identity its press is sensed under.
    pub id: Id,
    /// Whether a press can take it.
    pub how: Mention,
}

/// A measurement: its name on the left, and on the right its value in a box to
/// write in — or a dash, when the body does not carry it.
///
/// A measurement the body lacks gets no box. The document refuses a value for
/// a name its body does not carry, so a box there would take typing it could
/// never keep.
pub fn measure_row(
    ui: &mut Ui,
    theme: &Theme,
    named: &Named<'_>,
    field: Option<(Id, &Editable<'_>)>,
) -> (Response, Edited) {
    let fault = field.is_some_and(|(_, row)| row.fault);
    let height = if fault { LINE_H + FAULT_H } else { LINE_H };
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let middle = rect.top() + LINE_H / 2.0;
    let chip = mention(ui, theme, named, pos2(rect.left() + PAD, middle));
    let right = rect.right() - PAD - UNIT_W;
    let boxed = Rect::from_min_max(
        pos2(right - VALUE_W, middle - 12.0),
        pos2(right, middle + 12.0),
    );
    let Some((id, row)) = field else {
        ui.painter().text(
            boxed.right_center() - vec2(8.0, 0.0),
            Align2::RIGHT_CENTER,
            "—",
            FontId::monospace(12.0),
            theme.muted,
        );
        return (chip, Edited::Idle);
    };
    ui.painter().text(
        pos2(right + 6.0, middle),
        Align2::LEFT_CENTER,
        "cm",
        FontId::monospace(11.0),
        theme.muted,
    );
    if fault {
        let under = pos2(rect.right() - PAD, boxed.bottom() + 8.0);
        note(ui.painter(), theme, (under, Align2::RIGHT_CENTER), row);
    }
    (chip, edit_box(ui, theme, id, boxed, row))
}

/// A named formula: its name over a box as wide as the panel, and under the
/// box what it comes to.
///
/// Its name gets a line of its own because a pattern's names run long, and a
/// name laid beside its box would run under whatever was typed in it.
pub fn named_formula_row(
    ui: &mut Ui,
    theme: &Theme,
    named: &Named<'_>,
    id: Id,
    row: &Editable<'_>,
) -> (Response, Edited) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), NAMED_H), Sense::hover());
    let chip = mention(ui, theme, named, pos2(rect.left() + PAD, rect.top() + 13.0));
    let boxed = Rect::from_min_max(
        pos2(rect.left() + PAD, rect.top() + 26.0),
        pos2(rect.right() - PAD, rect.top() + 50.0),
    );
    let under = pos2(boxed.left() + 2.0, boxed.bottom() + 8.0);
    note(ui.painter(), theme, (under, Align2::LEFT_CENTER), row);
    (chip, edit_box(ui, theme, id, boxed, row))
}

/// A name whose text starts at `at`, vertically centred on it: a chip while a
/// press can take it, plain text while nothing can.
///
/// Both take the same room, so nothing on the panel shifts as a field takes
/// the focus. A live chip senses a click and never the focus, so a Tab out of
/// a formula lands on the next field and not on twenty names.
fn mention(ui: &mut Ui, theme: &Theme, named: &Named<'_>, at: Pos2) -> Response {
    let live = named.how == Mention::Live;
    let ink = if live { theme.accent } else { theme.ink_soft };
    let font = FontId::monospace(12.0);
    let galley = ui
        .painter()
        .layout_no_wrap(named.name.to_owned(), font, ink);
    let size = galley.size() + CHIP_PAD * 2.0;
    let rect = Rect::from_min_size(pos2(at.x - CHIP_PAD.x, at.y - size.y / 2.0), size);
    let sense = if live { Sense::CLICK } else { Sense::hover() };
    let resp = ui.interact(rect, named.id, sense);
    if live {
        let hovered = resp.hovered();
        let (fill, edge) = if hovered {
            (theme.accent.gamma_multiply(0.16), theme.accent)
        } else {
            (theme.raised, theme.accent.gamma_multiply(0.5))
        };
        ui.painter().rect(
            rect,
            CORNER,
            fill,
            Stroke::new(1.0, edge),
            StrokeKind::Inside,
        );
        if hovered {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
    }
    ui.painter().galley(rect.min + CHIP_PAD, galley, ink);
    resp
}
