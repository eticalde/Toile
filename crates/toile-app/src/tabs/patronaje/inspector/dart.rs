use eframe::egui::{self, Id};
use toile_engine::draft::{Command, DartKey, Draft, PieceKey};

use super::super::dart::{self, Cut, LET_GO, TAKE_OFF};
use super::super::wire::Verb;
use super::elastic::press;
use crate::theme::Theme;
use crate::widgets::{alert_note, field_row, footer_note, plain_note, section, section_with};

const NONE: &str = "Una pinza es la cuña que se saca del contorno más la costura que la cierra: \
                    con ella un borde recto se ajusta a un cuerpo que no lo es. Con la \
                    herramienta Pinza (D), pulsa las dos patas sobre un mismo tramo recto y luego \
                    el pico dentro de la pieza. Si la cuña ya está dibujada, pulsa sus tres nodos \
                    seguidos —pata, pico y pata— y la pinza se declara sobre ellos.";
const READ: &str = "La boca es lo que la pinza le quita al borde al cerrarse: un trasero cuya \
                    cintura mide de más se ajusta a la pretina con una pinza de esa diferencia. \
                    El largo de cada pata es lo que el recorrido gana en su lugar.";
const PRESS: &str = "Dejar de llamarla pinza quita el registro y la costura que cierra la cuña, y \
                     nada más: la cuña sigue dibujada, sin nada que diga que se cose. Hacia qué \
                     lado queda planchada lo decide la pata que se pulsa primero, y el documento \
                     no tiene ninguna edición que reescriba una pinza puesta: para cambiarlo, deja \
                     de llamarla pinza y vuelve a declararla empezando por la otra pata.";
const LOOSE: &str = "Dejar de llamarla pinza";
const TAKE: &str = "Quitar la pinza y su cuña";
const SHARED: &str = "Quitarla no se ofrece aquí: algo más del patrón se apoya en esos tres nodos \
                      —un piquete, una costura, una línea, un colgado o un eje de doblez—, y un \
                      archivo que los cita sin tenerlos no vuelve a abrirse. Saca de ahí lo que se \
                      apoya, o deja de llamarla pinza y la cuña se queda dibujada.";
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
    // The one that keeps the drawing goes first, and what the other one takes
    // is read before it is reached rather than regretted after: the record
    // cannot say whether this wedge was cut by the tool or drawn by
    // somebody, so the panel names what would go and lets the hand decide.
    if press(ui, theme, loosen_id(cut.dart), LOOSE) {
        verbs.extend(super::super::entry(
            LET_GO,
            Command::UndeclareDart { dart: cut.dart },
        ));
    }
    plain_note(ui, theme, &promises(cut));
    if cut.shared {
        plain_note(ui, theme, SHARED);
        return;
    }
    if press(ui, theme, take_off_id(cut.dart), TAKE) {
        verbs.extend(super::super::entry(
            TAKE_OFF,
            Command::RemoveDart { dart: cut.dart },
        ));
    }
}

/// The two promises side by side, with the three nodes named and the formulas
/// among their coordinates counted.
///
/// It sits between the presses because that is where it is read in time: the
/// destructive one has to say what it destroys before the hand reaches it. The
/// count is the one thing the panel can say about the cost without asking a
/// question the record cannot answer — whether the wedge was cut or drawn is
/// not written anywhere, but a formula is on the page either way, and a wedge
/// on plain numbers is a different promise that says so.
fn promises(cut: &Cut) -> String {
    let named = cut.names.iter().all(|name| !name.is_empty());
    let three = if named {
        format!(
            "«{}», «{}» y «{}»",
            cut.names[0], cut.names[1], cut.names[2]
        )
    } else {
        // A wedge the tool cut carries no names unless somebody wrote them, and
        // three empty pairs of quotes name nothing at all.
        "sus tres nodos".to_owned()
    };
    let keeps = format!("Dejar de llamarla pinza deja {three} dibujados como están.");
    match cut.formulas {
        0 => format!("{keeps} Quitarla se lleva los tres del dibujo."),
        n => format!(
            "{keeps} Quitarla se lleva los tres del dibujo, y con ellos las {n} fórmulas que \
             escriben sus coordenadas."
        ),
    }
}

/// The identity of the press that takes one dart off.
pub(in crate::tabs::patronaje) fn take_off_id(at: DartKey) -> Id {
    Id::new(("patronaje-dart-off", at))
}

/// The identity of the press that lets one go without taking its wedge.
pub(in crate::tabs::patronaje) fn loosen_id(at: DartKey) -> Id {
    Id::new(("patronaje-dart-loose", at))
}
