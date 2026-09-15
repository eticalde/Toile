use eframe::egui;
use toile_engine::draft::{Binding, Command, Draft};

use super::super::state::{Field, State};
use super::cite::{Cite, variable_id};
use super::write::{self, Asked};
use crate::theme::Theme;
use crate::widgets::{Editable, Named, named_formula_row, section_with};

const VARIABLE: &str = "editar variable";

/// The pattern's own quantities, at what they currently come to.
///
/// A variable is a name a formula reads exactly like a measurement, so its
/// name is offered the same way: pressed while a formula is being written, it
/// goes into that formula.
pub fn variables(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: &Draft,
    state: &mut State,
    cite: &Cite,
) -> Option<Asked> {
    let doc = draft.doc();
    section_with(ui, theme, "Variables", &doc.variables.len().to_string());
    let mut asked = None;
    for (key, variable) in doc.variables.iter() {
        let name = variable.name.as_str();
        let source = variable.value.source();
        let note = draft
            .env()
            .value(name)
            .map_or_else(|| "—".to_owned(), |value| format!("= {value:.2}"));
        let named = Named {
            name,
            id: variable_id(key),
            how: cite.variable(key),
        };
        let shown = Editable {
            label: name,
            source: &source,
            note: &note,
            fault: false,
            held: None,
        };
        let mut chip = None;
        let written = write::field(
            ui,
            state,
            cite,
            Field::Variable(key),
            &shown,
            |ui, id, row| {
                let (hit, edited) = named_formula_row(ui, theme, &named, id, row);
                chip = Some(hit);
                edited
            },
        );
        if chip.is_some_and(|hit| hit.clicked()) {
            cite.press(ui.ctx(), state, name);
        }
        if let Some(text) = written
            && let Ok(to) = Binding::parse(text.trim())
        {
            asked = Some((VARIABLE, Command::SetVariable { variable: key, to }));
        }
    }
    asked
}
