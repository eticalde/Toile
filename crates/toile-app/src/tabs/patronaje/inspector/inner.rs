use eframe::egui::{self, Id};
use toile_engine::draft::{Command, Draft, LineKey, LineKind, PieceKey};

use super::super::inner::{self, Drawn, ERASE};
use super::super::state::{Selection, State};
use super::super::wire::Verb;
use crate::theme::Theme;
use crate::widgets::{
    PAD, alert_note, button_named, cycle_named, field_row, list_row_named, plain_note, section,
    section_with,
};

const NONE: &str = "Con la herramienta Línea (L), pulsa donde sea sobre la pieza —el contorno o la \
                    tela— y luego el siguiente lugar. Enter cierra la línea, Esc la abandona y \
                    Retroceso quita el último lugar. Así se dibuja un doblez, un pespunte, una \
                    ranura o un ojal.";
const READ: &str = "El tipo dice qué se hace con la línea, y de ahí sale su trazo: el doblez se \
                    plancha, el pespunte se cose, la ranura y el ojal se abren, la posición dice \
                    qué cae ahí y la referencia no lleva nada. Supr borra la línea elegida.";
const DRAG: &str = "Los lugares sueltos de la línea elegida se arrastran de uno en uno, y llevan \
                    su fórmula. Un lugar anclado al contorno no: para moverlo se borra la línea y \
                    se vuelve a trazar.";
const ADRIFT: &str = "No se dibuja: alguno de sus lugares ya no resuelve. Revisa los puntos y el \
                      contorno que nombra.";
const UNDRAWN: &str = "sin dibujar";

const PURPOSE: &str = "cambiar el tipo de línea";

/// How wide the box that steps the kind is drawn.
const BOX_W: f32 = 150.0;

/// The lines drawn inside the piece, and the one chosen among them.
///
/// Nothing is written by looking, and nothing by choosing either: which line is
/// lit is a matter of view. The two controls under the list are one entry each.
pub fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: &Draft,
    piece: PieceKey,
    (state, verbs): (&mut State, &mut Vec<Verb>),
) {
    let drawn = inner::on(draft, piece);
    section_with(ui, theme, "Líneas internas", &drawn.len().to_string());
    if drawn.is_empty() {
        plain_note(ui, theme, NONE);
        return;
    }
    for it in &drawn {
        let label = format!("{} · {}", it.ordinal, word(it.kind));
        let lit = state.selection.line() == Some(it.line);
        let note = match (it.run.len() < 2, it.label.as_deref()) {
            (true, _) => (UNDRAWN.to_owned(), theme.alert),
            (false, Some(name)) => (name.to_owned(), theme.muted),
            (false, None) => ("—".to_owned(), theme.muted),
        };
        let row = (label.as_str(), lit);
        if list_row_named(ui, theme, row_id(it.line), row, (&note.0, note.1)).clicked() {
            // A press on the row already lit lets go of it.
            state.choose(if lit {
                Selection::None
            } else {
                Selection::Line(it.line)
            });
        }
    }
    let chosen = state
        .selection
        .line()
        .and_then(|key| drawn.iter().find(|it| it.line == key));
    if let Some(it) = chosen {
        detail(ui, theme, it, verbs);
    }
}

/// The chosen line: what it is for, how many places it runs through, and the
/// two presses that change it.
fn detail(ui: &mut egui::Ui, theme: &Theme, it: &Drawn, verbs: &mut Vec<Verb>) {
    section(ui, theme, &format!("Línea {}", it.ordinal));
    if let Some(name) = it.label.as_deref() {
        plain_note(ui, theme, name);
    }
    field_row(ui, theme, "lugares", &it.places.to_string(), "");
    field_row(ui, theme, "sueltos", &it.loose.len().to_string(), "");
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        let step = cycle_named(ui, theme, kind_id(it.line), "tipo", word(it.kind), BOX_W);
        if step.clicked() {
            verbs.extend(super::super::entry(
                PURPOSE,
                Command::SetLineKind {
                    line: it.line,
                    to: next(it.kind),
                },
            ));
        }
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        if button_named(ui, theme, erase_id(it.line), "Borrar línea").clicked() {
            verbs.extend(super::super::entry(
                ERASE,
                Command::RemoveLine { line: it.line },
            ));
        }
    });
    ui.add_space(4.0);
    if it.run.len() < 2 {
        alert_note(ui, theme, ADRIFT);
    }
    if !it.loose.is_empty() {
        plain_note(ui, theme, DRAG);
    }
    plain_note(ui, theme, READ);
}

/// What the panel calls each kind, in the words a pattern uses.
pub fn word(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Fold => "doblez",
        LineKind::Stitch => "pespunte",
        LineKind::Slit => "ranura",
        LineKind::Buttonhole => "ojal",
        LineKind::Placement => "posición",
        LineKind::Reference => "referencia",
    }
}

/// The kind a press steps to, in the order the document declares them.
///
/// A step and not a menu, because this app opens none: six kinds are few enough
/// that pressing round them is quicker than reading a list, and the box says so
/// with the cycle it carries.
fn next(kind: LineKind) -> LineKind {
    match kind {
        LineKind::Fold => LineKind::Stitch,
        LineKind::Stitch => LineKind::Slit,
        LineKind::Slit => LineKind::Buttonhole,
        LineKind::Buttonhole => LineKind::Placement,
        LineKind::Placement => LineKind::Reference,
        LineKind::Reference => LineKind::Fold,
    }
}

/// The identity egui keeps a line's row under: named for the line and never for
/// its place in the list.
pub(in crate::tabs::patronaje) fn row_id(at: LineKey) -> Id {
    Id::new(("patronaje-line-row", at))
}

/// The identity of the box that steps what a line is for.
pub(in crate::tabs::patronaje) fn kind_id(at: LineKey) -> Id {
    Id::new(("patronaje-line-kind", at))
}

/// The identity of the press that rubs a line out.
pub(in crate::tabs::patronaje) fn erase_id(at: LineKey) -> Id {
    Id::new(("patronaje-line-erase", at))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stepping the kind reaches every one of them and comes back.
    ///
    /// A cycle that missed one would leave a kind nothing in the app can ask
    /// for, and a file carrying it could be opened and never edited.
    #[test]
    fn the_step_reaches_every_kind_and_closes_the_ring() {
        let mut seen = vec![LineKind::Fold];
        let mut kind = LineKind::Fold;
        for _ in 0..6 {
            kind = next(kind);
            if kind == LineKind::Fold {
                break;
            }
            assert!(!seen.contains(&kind), "{kind:?} twice round");
            seen.push(kind);
        }
        assert_eq!(kind, LineKind::Fold, "the ring closes");
        assert_eq!(seen.len(), 6, "every kind is one press away: {seen:?}");
        let mut words: Vec<&str> = seen.iter().map(|&kind| word(kind)).collect();
        words.sort_unstable();
        words.dedup();
        assert_eq!(words.len(), 6, "and each is called something of its own");
    }
}
