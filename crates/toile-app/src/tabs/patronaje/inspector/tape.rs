use eframe::egui;
use toile_engine::draft::{Command, Doc, Draft, MannequinKey, MeasureSet};

use super::super::state::{Field, State};
use super::cite::{Cite, measure_id};
use super::write::{self, Asked};
use crate::tabs::UNNAMED;
use crate::theme::Theme;
use crate::widgets::{
    Editable, Named, PAD, cycle, footer_note, measure_row, readout, section, section_with,
};

const MEASURE: &str = "editar medida";
const BODY: &str = "cambiar de cuerpo";

/// The catalogue as a person reads it, under the headings the mannequin tab
/// gives the same groups.
const GROUPS: [(&str, &[&str]); 3] = [
    ("Contornos", &MeasureSet::GIRTHS),
    ("Largos y anchos", &MeasureSet::LENGTHS),
    ("Cuerpo", &MeasureSet::WHOLE),
];

/// Where a body's own names go, the ones the catalogue does not list.
const OTHERS: &str = "Otras";

/// One heading of the list, and each name under it with what the body makes
/// of it.
pub type Group<'a> = (&'static str, Vec<(&'a str, Option<f64>)>);

/// The measurements a formula can read: the body the pattern resolves with,
/// and every name, at what that body measures there.
///
/// The body box steps to the next mannequin of the document, one press at a
/// time, and says so with the cycle it carries. A document holding one body
/// has nowhere to step, so there it is drawn as what it is — the name of the
/// body the pattern resolves against — and senses no press at all.
pub fn measures(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: &Draft,
    state: &mut State,
    cite: &Cite,
) -> Option<Asked> {
    let doc = draft.doc();
    let set = doc.measures()?;
    section_with(ui, theme, "Medidas", "cm");
    let mut asked = body(ui, theme, doc, set);
    ui.add_space(4.0);
    footer_note(ui, theme, cite.hint());
    for (heading, names) in groups(set) {
        section(ui, theme, heading);
        for entry in names {
            asked = measure(ui, theme, (state, cite), doc.resolve_with, entry).or(asked);
        }
    }
    asked
}

/// Every name the list shows, under its heading.
///
/// The catalogue's names are always all there, carried or not: a pattern is
/// drafted for every body it may meet, so what it can read does not hang on
/// the one it resolves with today. A name the body carries outside the
/// catalogue follows under a heading of its own, so no value it holds is out
/// of reach.
pub fn groups(set: &MeasureSet) -> Vec<Group<'_>> {
    let mut groups: Vec<Group<'_>> = GROUPS
        .iter()
        .map(|&(heading, names)| (heading, names.iter().map(|&n| (n, set.get(n))).collect()))
        .collect();
    let others: Vec<_> = set
        .uncatalogued()
        .into_iter()
        .map(|name| (name, set.get(name)))
        .collect();
    if !others.is_empty() {
        groups.push((OTHERS, others));
    }
    groups
}

/// The box naming the body the pattern resolves with.
fn body(ui: &mut egui::Ui, theme: &Theme, doc: &Doc, set: &MeasureSet) -> Option<Asked> {
    let mut asked = None;
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        let named = if set.name.is_empty() {
            UNNAMED
        } else {
            &set.name
        };
        match next_body(doc) {
            None => readout(ui, theme, "resolver con", named, 170.0),
            Some(to) => {
                if cycle(ui, theme, "resolver con", named, 170.0).clicked() {
                    asked = Some((BODY, Command::ResolveWith { mannequin: to }));
                }
            }
        }
    });
    asked
}

/// One name of the list, with its value written in place when the body
/// carries one.
fn measure(
    ui: &mut egui::Ui,
    theme: &Theme,
    (state, cite): (&mut State, &Cite),
    mannequin: MannequinKey,
    (name, value): (&str, Option<f64>),
) -> Option<Asked> {
    let named = Named {
        name,
        id: measure_id(name),
        how: cite.measure(),
    };
    let mut chip = None;
    let written = if let Some(value) = value {
        let source = format!("{value:.1}");
        let shown = Editable {
            label: name,
            source: &source,
            note: "",
            fault: false,
            held: None,
        };
        let of = Field::Measure(name.to_owned());
        write::field(ui, state, cite, of, &shown, |ui, id, row| {
            let (hit, edited) = measure_row(ui, theme, &named, Some((id, row)));
            chip = Some(hit);
            edited
        })
    } else {
        chip = Some(measure_row(ui, theme, &named, None).0);
        None
    };
    if chip.is_some_and(|hit| hit.clicked()) {
        cite.press(ui.ctx(), state, name);
    }
    let to = written?.trim().parse::<f64>().ok()?;
    to.is_finite().then(|| {
        let name = name.to_owned();
        (
            MEASURE,
            Command::SetMeasure {
                mannequin,
                name,
                to,
            },
        )
    })
}

/// The next body in the document, when there is another one to try.
pub fn next_body(doc: &Doc) -> Option<MannequinKey> {
    let bodies: Vec<MannequinKey> = doc.mannequins.keys().collect();
    if bodies.len() < 2 {
        return None;
    }
    let at = bodies.iter().position(|&key| key == doc.resolve_with)?;
    bodies.get((at + 1) % bodies.len()).copied()
}
