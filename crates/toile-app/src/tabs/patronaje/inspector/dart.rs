use eframe::egui::{self, Id};
use toile_engine::draft::{Command, DartKey, Draft, PieceKey};

use super::super::dart::{self, Cut, TAKE_OFF};
use super::super::wire::Verb;
use super::elastic::press;
use crate::theme::Theme;
use crate::widgets::{alert_note, field_row, footer_note, plain_note, section, section_with};

const NONE: &str = "Una pinza es la cuña que se saca del contorno más la costura que la cierra: \
                    con ella un borde recto se ajusta a un cuerpo que no lo es. Con la \
                    herramienta Pinza (D), pulsa las dos patas sobre un mismo tramo recto y luego \
                    el pico dentro de la pieza.";
const READ: &str = "La boca es lo que la pinza le quita al borde al cerrarse: un trasero cuya \
                    cintura mide de más se ajusta a la pretina con una pinza de esa diferencia. \
                    El largo de cada pata es lo que el recorrido gana en su lugar.";
const PRESS: &str = "Hacia qué lado queda la cuña planchada lo decide la pata que se pulsa \
                     primero. Para cambiarlo se quita la pinza y se vuelve a cortar: el \
                     documento no tiene ninguna edición que reescriba una pinza puesta.";
const ADRIFT: &str = "No se dibuja: alguno de sus tres puntos ya no resuelve. Revisa las \
                      fórmulas de las patas y del pico.";

/// The darts cut into the piece, each with the press that takes it off.
///
/// Nothing is written by looking: opening a product and reading this section
/// leaves its bytes and its version exactly as they were. Every dart of the
/// piece is laid out in full rather than listed and then chosen, because a
/// piece carries one or two of them and a row that has to be pressed before it
/// says anything would hide the numbers a waistband is matched against.
pub fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: &Draft,
    piece: PieceKey,
    verbs: &mut Vec<Verb>,
) {
    let cuts = dart::on(draft, piece);
    section_with(ui, theme, "Pinzas", &cuts.len().to_string());
    if cuts.is_empty() {
        plain_note(ui, theme, NONE);
        return;
    }
    for cut in &cuts {
        detail(ui, theme, cut, verbs);
    }
    plain_note(ui, theme, PRESS);
    footer_note(ui, theme, READ);
}

/// One dart: which it is, what it measures, which way it is pressed, and the
/// press that takes it off.
fn detail(ui: &mut egui::Ui, theme: &Theme, cut: &Cut, verbs: &mut Vec<Verb>) {
    section(ui, theme, &format!("Pinza {}", cut.ordinal));
    let legs = format!("«{}» y «{}»", cut.names[0], cut.names[2]);
    field_row(ui, theme, "patas", &legs, "");
    field_row(ui, theme, "pico", &format!("«{}»", cut.names[1]), "");
    match (cut.mouth_cm(), cut.legs_cm()) {
        (Some(mouth), Some(long)) => {
            field_row(ui, theme, "boca", &format!("{mouth:.2}"), "cm");
            let each = format!("{:.2} · {:.2}", long[0], long[1]);
            field_row(ui, theme, "largo", &each, "cm");
        }
        _ => alert_note(ui, theme, ADRIFT),
    }
    field_row(
        ui,
        theme,
        "planchada hacia",
        &format!("«{}»", cut.toward()),
        "",
    );
    if press(ui, theme, take_off_id(cut.dart), "Quitar pinza") {
        verbs.extend(super::super::entry(
            TAKE_OFF,
            Command::RemoveDart { dart: cut.dart },
        ));
    }
}

/// The identity of the press that takes one dart off.
pub(in crate::tabs::patronaje) fn take_off_id(at: DartKey) -> Id {
    Id::new(("patronaje-dart-off", at))
}
