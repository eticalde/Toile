use eframe::egui;
use toile_engine::session::Session;

use crate::file::{self, Action, File};
use crate::tabs::{self, Tab};
use crate::theme::Theme;

/// Height of the tab bar, in points.
const TOPBAR_H: f32 = 40.0;
/// Height of the status bar, in points.
const STATUS_H: f32 = 26.0;
/// Inset of the first item in either bar, in points.
const BAR_INSET: i8 = 16;

/// Both bars fill themselves, so egui's own separator line is off: it would
/// reserve a point of the bar's height and then paint over the tab underline
/// that lands in it.
fn bar_frame(theme: &Theme) -> egui::Frame {
    egui::Frame::new()
        .fill(theme.panel)
        .inner_margin(egui::Margin::symmetric(BAR_INSET, 0))
}

pub fn top(
    ui: &mut egui::Ui,
    theme: &Theme,
    tab: &mut Tab,
    held: &File,
    revision: u64,
) -> Option<Action> {
    egui::Panel::top("topbar")
        .exact_size(TOPBAR_H)
        .show_separator_line(false)
        .frame(bar_frame(theme))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for stage in Tab::ALL {
                    if tab_item(ui, theme, stage.label(), *tab == stage).clicked() {
                        *tab = stage;
                    }
                }
                file::bar(ui, theme, held, revision)
            })
            .inner
        })
        .inner
}

/// One stage of the pipeline; the open one is underlined in the accent.
fn tab_item(ui: &mut egui::Ui, theme: &Theme, label: &str, active: bool) -> egui::Response {
    let ink = if active { theme.ink } else { theme.muted };
    let text = ui
        .painter()
        .layout_no_wrap(label.to_owned(), egui::FontId::proportional(13.0), ink);
    let size = egui::vec2(text.size().x + 28.0, ui.available_height());
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    if resp.hovered() && !active {
        ui.painter()
            .rect_filled(rect, 0.0, theme.accent.gamma_multiply(0.07));
    }
    let at = rect.center() - text.size() / 2.0;
    ui.painter().galley(at, text, ink);
    if active {
        let underline = egui::Rect::from_min_max(
            egui::pos2(rect.left(), rect.bottom() - 2.0),
            rect.right_bottom(),
        );
        ui.painter().rect_filled(underline, 0.0, theme.accent);
    }
    resp
}

pub fn status(
    ui: &mut egui::Ui,
    theme: &Theme,
    tab: Tab,
    session: &Session,
    patronaje: &tabs::patronaje::State,
) {
    egui::Panel::bottom("status")
        .exact_size(STATUS_H)
        .show_separator_line(false)
        .frame(bar_frame(theme))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for (i, cell) in tab.status(session, patronaje).iter().enumerate() {
                    if i > 0 {
                        ui.label(cell_text(" · ", theme.line));
                    }
                    ui.label(cell_text(cell, theme.muted));
                }
            });
        });
}

fn cell_text(text: &str, color: egui::Color32) -> egui::RichText {
    egui::RichText::new(text)
        .monospace()
        .size(11.0)
        .color(color)
}
