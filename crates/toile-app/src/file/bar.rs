use eframe::egui::{self, Align, Layout, RichText};

use super::{Action, File};
use crate::theme::Theme;

/// The mark a pattern with unwritten changes carries after its name.
const DIRTY: &str = " •";

/// The three ways to move the file, right to left along the top bar.
const MOVES: [(&str, Action); 3] = [
    ("Guardar como", Action::SaveAs),
    ("Guardar", Action::Save),
    ("Abrir", Action::Open),
];

/// The file's side of the top bar: what the pattern is called, whether it has
/// changed since it was written, and the buttons that move it.
///
/// The name is the answer to "which pattern am I looking at", so it sits in
/// the bar and not in a menu, and the mark beside it is the answer to "would I
/// lose anything by closing this".
pub fn show(ui: &mut egui::Ui, theme: &Theme, file: &File, revision: u64) -> Option<Action> {
    let dirty = file.dirty(revision);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let mut asked = None;
        for (label, action) in MOVES {
            if crate::widgets::button_secondary(ui, theme, label).clicked() {
                asked = Some(action);
            }
        }
        ui.add_space(6.0);
        let name = format!("{}{}", file.name(), if dirty { DIRTY } else { "" });
        let ink = if dirty { theme.accent } else { theme.ink_soft };
        ui.label(RichText::new(name).monospace().size(11.0).color(ink));
        if let Some(notice) = file.notice(revision) {
            ui.add_space(6.0);
            let ink = if notice.bad { theme.alert } else { theme.muted };
            ui.label(
                RichText::new(&notice.text)
                    .monospace()
                    .size(11.0)
                    .color(ink),
            );
        }
        asked
    })
    .inner
}
