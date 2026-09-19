use eframe::egui::{self, Color32, Id};
use toile_engine::draft::{
    Command, Draft, EdgeRange, Identity, Seam, SeamKey, SeamKind, SeamOrientation,
};
use toile_engine::session::SeamFault;

use super::super::state::{Selection, State};
use super::super::wire::Verb;
use crate::seam::{self, Lengths};
use crate::theme::Theme;
use crate::widgets::{
    PAD, alert_note, button_named, field_row, list_row_named, plain_note, section, section_with,
};

const NONE: &str = "Con la herramienta Coser (S), pulsa un tramo de una pieza y luego el tramo \
                    al que va cosido. Esc suelta el primero.";
const READ: &str = "Las flechas de la mesa marcan el sentido en que se cosen los dos lados, desde \
                    el punto relleno. Si los dos hilos finos que unen sus extremos se cruzan, la \
                    prenda sale torcida: invierte el sentido.";
const UNPAIRED: &str = "no llega a la tela";

const FLIP: &str = "invertir el sentido de la costura";
const UNPICK: &str = "descoser";

/// The seams of the product, and the one chosen among them.
///
/// Nothing is written by looking, and nothing by choosing either: which seam
/// is lit is a matter of view. The two presses that edit are one entry each.
pub fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: &Draft,
    faults: &[(SeamKey, SeamFault)],
    (state, verbs): (&mut State, &mut Vec<Verb>),
) {
    let doc = draft.doc();
    section_with(ui, theme, "Costuras", &doc.seams.len().to_string());
    if doc.seams.is_empty() {
        plain_note(ui, theme, NONE);
        return;
    }
    let fault = |key: SeamKey| faults.iter().find(|(at, _)| *at == key).map(|(_, why)| why);
    for (index, (key, held)) in doc.seams.iter().enumerate() {
        let label = format!("{} · {}", index + 1, pieces(draft, held));
        let note = verdict(theme, draft, held, fault(key).is_some());
        let lit = state.selection.seam() == Some(key);
        let row = (label.as_str(), lit);
        if list_row_named(ui, theme, row_id(key), row, (&note.0, note.1)).clicked() {
            // A press on the row already lit lets go of it.
            state.choose(if lit {
                Selection::None
            } else {
                Selection::Seam(key)
            });
        }
    }
    let chosen = state.selection.seam().and_then(|key| {
        let at = doc.seams.keys().position(|it| it == key)?;
        Some((at + 1, key, *doc.seams.get(key)?))
    });
    if let Some((ordinal, key, held)) = chosen {
        detail(ui, theme, draft, (ordinal, key, &held), fault(key), verbs);
    }
}

/// The chosen seam: what each side measures, how they compare, which way they
/// are paired, and the two presses that change it.
fn detail(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: &Draft,
    (ordinal, key, held): (usize, SeamKey, &Seam),
    fault: Option<&SeamFault>,
    verbs: &mut Vec<Verb>,
) {
    section(ui, theme, &format!("Costura {ordinal}"));
    plain_note(ui, theme, &named(draft, held));
    if let Some(sides) = Lengths::of(draft, held) {
        field_row(ui, theme, "lado A", &format!("{:.1}", sides.a), "cm");
        field_row(ui, theme, "lado B", &format!("{:.1}", sides.b), "cm");
        field_row(ui, theme, "A − B", &format!("{:+.1}", sides.delta()), "cm");
        let (said, meets) = compared(draft, held, sides);
        if meets == Some(false) {
            alert_note(ui, theme, &said);
        } else {
            plain_note(ui, theme, &said);
        }
    }
    let way = match held.orientation {
        SeamOrientation::Aligned => "mismo",
        SeamOrientation::Opposed => "contrario",
    };
    field_row(ui, theme, "sentido", way, "");
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_space(PAD);
        if button_named(ui, theme, flip_id(key), "Invertir sentido").clicked() {
            verbs.extend(flipped(key, held));
        }
        if button_named(ui, theme, unpick_id(key), "Descoser").clicked() {
            verbs.extend(super::super::entry(
                UNPICK,
                Command::RemoveSeam { seam: key },
            ));
        }
    });
    ui.add_space(4.0);
    if let Some(why) = fault {
        let said = format!("No llega a la tela: {}.", seam::why(why));
        alert_note(ui, theme, &said);
    }
    plain_note(ui, theme, READ);
}

/// The seam taken out and put back under its own key the other way round, as
/// one entry.
///
/// The document has no edit that turns a seam over, and it does not need one:
/// the key is what everything else knows the seam by, and it survives.
fn flipped(key: SeamKey, held: &Seam) -> Vec<Verb> {
    let orientation = match held.orientation {
        SeamOrientation::Aligned => SeamOrientation::Opposed,
        SeamOrientation::Opposed => SeamOrientation::Aligned,
    };
    let seam = Seam {
        orientation,
        ..*held
    };
    let back = Command::AddSeam {
        identity: Identity::Restored(key),
        seam,
    };
    vec![
        Verb::Begin(FLIP),
        Verb::Edit(Box::new(Command::RemoveSeam { seam: key })),
        Verb::Edit(Box::new(back)),
        Verb::End,
    ]
}

/// The two pieces a seam joins, by name.
fn pieces(draft: &Draft, held: &Seam) -> String {
    let name = |range: EdgeRange| {
        let piece = draft.doc().pieces.get(range.piece()?)?;
        Some(piece.name.clone())
    };
    let (a, b) = (name(held.a), name(held.b));
    let or_dash = |name: Option<String>| name.unwrap_or_else(|| "—".to_owned());
    format!("{} — {}", or_dash(a), or_dash(b))
}

/// Each side spelt out: its piece, and the two nodes it runs between.
fn named(draft: &Draft, held: &Seam) -> String {
    let side = |letter: &str, range: EdgeRange| {
        let piece = range
            .piece()
            .and_then(|key| draft.doc().pieces.get(key))
            .map_or("—", |piece| piece.name.as_str());
        let run = seam::name(draft, &range).unwrap_or_else(|| "sin anclar".to_owned());
        format!("{letter} · {piece} · {run}")
    };
    format!("{}\n{}", side("A", held.a), side("B", held.b))
}

/// What a row says of its seam at a glance: the difference in length, in the
/// ink of a fault when it is outside the tolerance or never reached the cloth.
fn verdict(theme: &Theme, draft: &Draft, held: &Seam, unpaired: bool) -> (String, Color32) {
    if unpaired {
        return (UNPAIRED.to_owned(), theme.alert);
    }
    let Some(sides) = Lengths::of(draft, held) else {
        return ("—".to_owned(), theme.muted);
    };
    let ink = match sides.meets(draft, held) {
        Some(false) => theme.alert,
        Some(true) | None => theme.muted,
    };
    (format!("Δ {:+.1}", sides.delta()), ink)
}

/// The difference in words: which side is left over and by how much, and how
/// that stands against what the seam allows.
///
/// The percentage is of the shorter side, the one the longer is eased onto.
fn compared(draft: &Draft, held: &Seam, sides: Lengths) -> (String, Option<bool>) {
    let delta = sides.delta();
    let shorter = sides.a.min(sides.b);
    let over = if delta.abs() < 0.05 {
        "Los dos lados miden lo mismo".to_owned()
    } else {
        let side = if delta > 0.0 { "A" } else { "B" };
        let percent = if shorter > 0.0 {
            delta.abs() / shorter * 100.0
        } else {
            0.0
        };
        format!(
            "Sobra el lado {side}: {:.1} cm ({percent:.1} %)",
            delta.abs()
        )
    };
    let meets = sides.meets(draft, held);
    let tolerance = seam::tolerance_cm(draft, held);
    let rule = match (held.kind, meets) {
        (SeamKind::Gathered { ratio }, _) => {
            format!("fruncida a razón {ratio:.2}: se juzga por la razón, no por la diferencia")
        }
        (SeamKind::Eased { expected_cm }, Some(true)) => {
            format!("embebida de {expected_cm:.1} cm, dentro de la tolerancia de {tolerance:.1} cm")
        }
        (SeamKind::Eased { expected_cm }, _) => {
            format!("embebida de {expected_cm:.1} cm, fuera de la tolerancia de {tolerance:.1} cm")
        }
        (SeamKind::Plain, Some(false)) => format!("fuera de la tolerancia de {tolerance:.1} cm"),
        (SeamKind::Plain, _) => format!("dentro de la tolerancia de {tolerance:.1} cm"),
    };
    (format!("{over} · {rule}."), meets)
}

/// The identity egui keeps a seam's row under: named for the seam and never
/// for its place in the list.
pub(in crate::tabs::patronaje) fn row_id(at: SeamKey) -> Id {
    Id::new(("patronaje-seam-row", at))
}

/// The identity of the press that turns a seam over.
pub(in crate::tabs::patronaje) fn flip_id(at: SeamKey) -> Id {
    Id::new(("patronaje-seam-flip", at))
}

/// The identity of the press that unpicks one.
pub(in crate::tabs::patronaje) fn unpick_id(at: SeamKey) -> Id {
    Id::new(("patronaje-seam-unpick", at))
}
