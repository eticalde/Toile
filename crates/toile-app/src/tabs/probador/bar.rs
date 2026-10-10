use std::ops::Range;

use eframe::egui;
use toile_engine::session::Session;

use super::fitted;
use super::icons::{pause_icon, play_icon, reset_icon};
use crate::theme::Theme;
use crate::widgets::{button_ghost_icon, ghost_icon_room, readout, readout_room};

/// How tall one row of the bar is drawn, in points.
///
/// One row and not the whole bar, because the register wraps: see [`rows`].
pub const SUBBAR_H: f32 = 44.0;

/// How far in from either end the bar's frame holds its contents, in points.
const INSET: i8 = 16;

/// How wide the box naming the body the document resolves against is drawn.
const FITTED_W: f32 = 150.0;

/// And the box saying what solving that body cost.
const BODY_W: f32 = 170.0;

/// And the box saying where the body crosses itself: a count and a place.
const CROSSED_W: f32 = 190.0;

/// How wide the box that says what the body is holding up is drawn: enough for
/// the longest reading it can carry without the words being cut.
const HUNG_W: f32 = 210.0;
/// And the box naming the two rings a release had to choose between: two
/// catalogue names and the word between them.
const WORN_W: f32 = 230.0;

/// And the box counting the pieces no ring could place: a number and three
/// words.
const ADRIFT_W: f32 = 150.0;

/// And the box saying how much bigger than its cloth a ring came off: a ratio,
/// two words and a catalogue name.
const LOOSE_W: f32 = 220.0;

/// What paints one control's mark into the slot its button hands over.
type Glyph = fn(&egui::Painter, egui::Rect, egui::Color32);

/// The three sim controls, right to left as they are drawn, with the glyph each
/// carries.
const SIM: [(&str, Glyph); 3] = [
    ("Reiniciar", reset_icon),
    ("Pausar", pause_icon),
    ("Simular", play_icon),
];

/// How far apart the three are drawn, in points.
const SIM_GAP: f32 = 6.0;

/// What the register has to say about one drape.
///
/// A struct and not a tuple of five, because every one of them is an
/// `Option<&str>` and nothing but the name would say which is which.
#[derive(Default, Clone, Copy)]
pub struct Read<'a> {
    /// Where the body crosses itself, when a garment can reach it.
    pub crossed: Option<&'a str>,
    /// How near its rings the body is holding the garment up.
    pub hanging: Option<&'a str>,
    /// Which ring the product was let go on, when that is not the one its own
    /// size points at.
    pub worn: Option<&'a str>,
    /// How much bigger than the cloth on it a ring had to be opened, when one
    /// came off much bigger.
    pub loose: Option<&'a str>,
    /// How many of its pieces no ring could place.
    pub adrift: Option<&'a str>,
}

/// One box of the register as the row lays it out.
struct Item<'a> {
    caption: &'a str,
    value: &'a str,
    width: f32,
}

/// The bar over the table: the body being fitted, and the sim controls.
///
/// The body is the only one of the three things a fitting names that the
/// document can answer for. It carries no product name, and the app carries no
/// fabric at all, so a box for either would read the same two words over every
/// pattern ever opened. It is a readout and not a picker, because the document
/// resolves against the body the drafting table chose and this bar has no say.
///
/// The three sim controls are drawn dead. The sim thread takes a rest update,
/// a swapped mesh and a shutdown, and nothing else: there is no pause to ask
/// for, no resume, and no starting state to go back to. They keep their room
/// so that the phase which builds them moves nothing on this bar.
///
/// `read` is what the register has to say about this drape. Each reading is
/// `None` when nothing of the kind was measured, and then takes no room here.
pub fn sub_bar(ui: &mut egui::Ui, theme: &Theme, session: &Session, body: &str, read: Read<'_>) {
    let items = register(session, body, read);
    let gap = ui.spacing().item_spacing.x;
    // Asked of the widgets that draw them and never worked out here: see
    // `readout_room`. Both readings are taken against the ui the panel is
    // about to be built in, which is the one whose fonts it will paint with.
    let rooms: Vec<f32> = items
        .iter()
        .map(|it| readout_room(ui, theme, it.caption, it.width))
        .collect();
    let sim: f32 = SIM
        .iter()
        .map(|&(label, _)| ghost_icon_room(ui, theme, label))
        .sum::<f32>()
        + SIM_GAP * (SIM.len() - 1) as f32;
    let full = ui.available_width() - 2.0 * f32::from(INSET);
    let lines = rows(&rooms, gap, full, full - sim - gap);
    egui::Panel::top("probador-subbar")
        .exact_size(SUBBAR_H * lines.len() as f32)
        .frame(
            egui::Frame::new()
                .fill(theme.panel)
                .inner_margin(egui::Margin::symmetric(INSET, 0)),
        )
        .show(ui, |ui| {
            // Rows butt against each other, so the bar is as tall as the rows
            // it holds and `exact_size` above is the truth about it.
            ui.spacing_mut().item_spacing.y = 0.0;
            let wide = ui.available_width();
            for (n, line) in lines.iter().enumerate() {
                let (_, strip) = ui.allocate_space(egui::vec2(wide, SUBBAR_H));
                ui.scope_builder(egui::UiBuilder::new().max_rect(strip), |ui| {
                    ui.horizontal_centered(|ui| {
                        for it in &items[line.clone()] {
                            readout(ui, theme, it.caption, it.value, it.width);
                        }
                        if n == 0 {
                            sim_controls(ui, theme);
                        }
                    });
                });
            }
        });
}

/// Every box the register has to draw, in the order it reads: the body being
/// fitted first, then what was measured about this drape.
fn register<'a>(session: &'a Session, body: &'a str, read: Read<'a>) -> Vec<Item<'a>> {
    let mut all = Vec::new();
    if let Some(named) = fitted(session) {
        all.push(Item {
            caption: "maniquí",
            value: named,
            width: FITTED_W,
        });
    }
    all.push(Item {
        caption: "cuerpo",
        value: body,
        width: BODY_W,
    });
    // In the ink of any other readout, and in this order: every one of them is
    // a thing measured about this drape, and a drape that measured badly is
    // still a drape. None of them is a warning and none is drawn as one.
    for (caption, said, width) in [
        ("cruces", read.crossed, CROSSED_W),
        ("colgado", read.hanging, HUNG_W),
        ("estación", read.worn, WORN_W),
        ("aro", read.loose, LOOSE_W),
        ("sin sitio", read.adrift, ADRIFT_W),
    ] {
        if let Some(value) = said {
            all.push(Item {
                caption,
                value,
                width,
            });
        }
    }
    all
}

/// Which boxes go on each row, given the room each takes and the room the rows
/// have: `first` for the first, which is shorter by the sim controls, `full`
/// for the rest.
///
/// The bar wraps rather than cutting or dropping anything, which is the
/// decision. The window the app opens is 1320 pt wide and leaves 1288 inside
/// the frame; the whole register asks for 1748 of that and the sim controls for
/// 271 more. On one line, measured at 1320: `aro` wrote its value at 1324,
/// `sin sitio` wrote its at 1624 and its caption nowhere — egui does not paint
/// a label it finds off the clip — and all three controls stood outside. At
/// 1512 four of those were still out. A reading nobody can see is worth less
/// than the row it would have cost.
///
/// Greedy and in reading order, because the order is the sentence: the body
/// comes first and every box after it is about this drape. One box per row at
/// the very least, so a window narrower than a single box still draws it.
fn rows(rooms: &[f32], gap: f32, full: f32, first: f32) -> Vec<Range<usize>> {
    let mut lines: Vec<Range<usize>> = Vec::new();
    let (mut at, mut used) = (0, 0.0);
    for (k, &room) in rooms.iter().enumerate() {
        let room_for = if lines.is_empty() { first } else { full };
        let want = if k == at { room } else { used + gap + room };
        if k > at && want > room_for {
            lines.push(at..k);
            (at, used) = (k, room);
        } else {
            used = want;
        }
    }
    lines.push(at..rooms.len());
    lines
}

/// The three dead sim controls, at the right-hand end of the row they are on.
fn sim_controls(ui: &mut egui::Ui, theme: &Theme) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = SIM_GAP;
        for (label, icon) in SIM {
            button_ghost_icon(ui, theme, label, icon);
        }
    });
}
