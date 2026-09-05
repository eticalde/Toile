use eframe::egui::{self, Align2, FontId, Rect, Response, Sense, Stroke, StrokeKind, Vec2, vec2};

use super::state::{State, Tool};
use super::wire::Verb;
use crate::glyph;
use crate::theme::Theme;
use crate::widgets::{CORNER, PAD, section};

/// The nine tools: the name, the icon that stands for it, and whether the
/// program can already do it. A tile that cannot is drawn dead and answers
/// nothing, because a button that lies is worse than a gap.
const TOOLS: [(&str, &str, bool); 9] = [
    ("Seleccionar", "3 2 13 8 8.8 9 7 13 3 2", true),
    ("Punto", "o 8 8 2.5", true),
    (
        "Recta",
        "3.5 12.5 12.5 3.5; o 3.5 12 1.4; o 12 3.5 1.4",
        true,
    ),
    (
        "Curva",
        "2 13 5 12 8 9 11 4 14 3; 2 13 5 9; o 5 9 1.3",
        true,
    ),
    ("Pinza", "3 3 8 13 13 3", false),
    ("Piquete", "2 9 14 9; 8 9 8 5", false),
    ("Espejo", "8 2 8 14; 5 5 2 8 5 11; 11 5 14 8 11 11", false),
    ("Medir", "2 6 14 6 14 10 2 10 2 6; 5 6 5 8; 11 6 11 8", true),
    ("Coser", "2 11 5 6 8 11 11 6 14 11", false),
];

/// The tile that is a switch and not a tool: measuring is never modal, so it
/// only decides whether every tract carries its length or just the one under
/// the pointer.
const MEASURE: &str = "Medir";

/// The tool a tile puts in hand, for the tiles whose phase has arrived.
fn tool_of(name: &str) -> Option<Tool> {
    match name {
        "Seleccionar" => Some(Tool::Select),
        "Punto" => Some(Tool::Point),
        "Recta" => Some(Tool::Line),
        "Curva" => Some(Tool::Curve),
        _ => None,
    }
}

/// The two steps through the undo stack, and the arrows that stand for them.
const HISTORY: [(&str, &str); 2] = [
    ("Deshacer", "3 7 12 7 12 12; 3 7 6 4; 3 7 6 10"),
    ("Rehacer", "13 7 4 7 4 12; 13 7 10 4; 13 7 10 10"),
];

/// The tool grid, three tiles to a row, with the one in hand lit.
pub fn grid(ui: &mut egui::Ui, theme: &Theme, state: &mut State) {
    section(ui, theme, "Herramientas");
    let width = tile_width(ui);
    for tools in TOOLS.chunks(3) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.add_space(PAD);
            for &(name, icon, ready) in tools {
                let held = tool_of(name);
                let lit = if name == MEASURE {
                    state.dimensions
                } else {
                    held == Some(state.tool)
                };
                let resp = tile(ui, theme, name, icon, Weight::of(lit, ready), width);
                if !resp.clicked() {
                    continue;
                }
                if name == MEASURE {
                    state.dimensions = !state.dimensions;
                } else if let Some(chosen) = held {
                    state.tool = chosen;
                }
            }
        });
        ui.add_space(4.0);
    }
}

/// The two steps through the undo stack, each dead while it leads nowhere.
pub fn history(ui: &mut egui::Ui, theme: &Theme, ready: [bool; 2]) -> Option<Verb> {
    section(ui, theme, "Historial");
    let width = tile_width(ui);
    let mut asked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.add_space(PAD);
        for (index, &(name, icon)) in HISTORY.iter().enumerate() {
            let weight = Weight::of(false, ready[index]);
            if tile(ui, theme, name, icon, weight, width).clicked() && ready[index] {
                asked = Some(if index == 0 { Verb::Undo } else { Verb::Redo });
            }
        }
    });
    ui.add_space(4.0);
    asked
}

/// The width three tiles take across the panel.
fn tile_width(ui: &egui::Ui) -> f32 {
    (ui.available_width() - 2.0 * PAD - 8.0) / 3.0
}

/// How a tile is drawn: the one in hand, one waiting to be picked up, or one
/// whose phase has not arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Weight {
    Held,
    Ready,
    Absent,
}

impl Weight {
    fn of(held: bool, ready: bool) -> Weight {
        match (held, ready) {
            (true, _) => Weight::Held,
            (false, true) => Weight::Ready,
            (false, false) => Weight::Absent,
        }
    }
}

fn tile(
    ui: &mut egui::Ui,
    theme: &Theme,
    name: &str,
    icon: &str,
    weight: Weight,
    width: f32,
) -> Response {
    let sense = if weight == Weight::Absent {
        Sense::hover()
    } else {
        Sense::click()
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 48.0), sense);
    let p = ui.painter();
    let ink = match weight {
        Weight::Held => theme.ink,
        Weight::Ready => theme.ink_soft,
        Weight::Absent => theme.muted,
    };
    if weight == Weight::Held {
        p.rect_filled(rect, CORNER, theme.accent.gamma_multiply(0.16));
    } else if resp.hovered() {
        p.rect_filled(rect, CORNER, theme.accent.gamma_multiply(0.07));
    }
    if weight != Weight::Absent {
        let edge = if weight == Weight::Held {
            theme.accent
        } else {
            theme.line
        };
        p.rect_stroke(rect, CORNER, Stroke::new(1.0, edge), StrokeKind::Inside);
    }
    let slot = Rect::from_center_size(rect.center_top() + vec2(0.0, 17.0), Vec2::splat(16.0));
    glyph::paint(p, slot, ink, icon);
    let at = rect.center_bottom() - vec2(0.0, 12.0);
    let font = FontId::proportional(10.0);
    p.text(at, Align2::CENTER_CENTER, name, font, ink);
    resp
}
