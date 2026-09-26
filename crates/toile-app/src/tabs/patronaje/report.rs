use toile_engine::draft::{Draft, PieceKey};
use toile_engine::session::Session;

use super::gesture::Gesture;
use super::state::{Scope, State, Tool};
use super::{active_piece, dart};

/// The two nodes a base block names for the side seam, so the bar can measure
/// it instead of quoting it. A piece that names neither reports its perimeter.
const SIDE: [&str; 2] = ["cintura_lat", "bajo_lat"];

/// The cells of the status bar, measured off the document on the table.
///
/// Each cell says whether it is an alert: a refused edit and a broken contour
/// are painted to be seen, and everything else stays quiet.
pub fn status(session: &Session, state: &State) -> Vec<(String, bool)> {
    // While a piece is being drawn the bar tells how to finish it: with no
    // menu and no tool tile any more, this is where the two keys are named.
    if matches!(state.gesture, Gesture::Drawing { .. }) {
        return vec![
            ("dibujando pieza".to_owned(), false),
            ("Enter cierra · Esc cancela".to_owned(), false),
            ("cm".to_owned(), false),
        ];
    }
    // A line being traced is the same kind of promise, and the same two keys
    // finish or abandon it, so it is said in the same place.
    if let Gesture::Tracing(held) = &state.gesture {
        return vec![
            (
                format!("trazando línea · {} lugares", held.pending.len()),
                false,
            ),
            (
                "Enter cierra · Esc cancela · Retroceso deshace".to_owned(),
                false,
            ),
            ("cm".to_owned(), false),
        ];
    }
    // A wedge being cut is a third promise of the same kind, and the bar is
    // where the press that has not happened yet is named — including which way
    // the finished dart will be pressed, which is decided by the leg pressed
    // first and so has to be readable before the apex goes down.
    if let Gesture::Darting(held) = &state.gesture {
        let legs = held.legs.len();
        let next = if legs < dart::LEGS {
            "pulsa la segunda pata en el mismo tramo"
        } else {
            "pulsa el pico dentro de la pieza"
        };
        return vec![
            (
                format!("cortando pinza · {legs} de {} patas", dart::LEGS),
                false,
            ),
            (next.to_owned(), false),
            (
                "se plancha hacia la primera pata · Retroceso quita la última · Esc cancela"
                    .to_owned(),
                false,
            ),
            ("cm".to_owned(), false),
        ];
    }
    let draft = session.draft();
    let mut cells = match (draft, state.scope) {
        (Some(draft), Scope::Product) => product(draft, state),
        (Some(draft), Scope::Piece) => {
            match active_piece(Some(draft), state.active, session.piece()) {
                Some(piece) => one(draft, piece),
                None => return empty(),
            }
        }
        (None, _) => return empty(),
    };
    // The tile says which tool is in hand; what a press with it is aimed at is
    // said here, where a piece tool that waits for a target has room to say so.
    if state.scope == Scope::Piece && state.tool == Tool::Dart {
        let said = "pinza: pulsa la primera pata en un tramo recto";
        cells.push((said.to_owned(), false));
    }
    if let Some(why) = state.refused.as_deref() {
        cells.push((format!("rechazado: {why}"), true));
    }
    if let Some(label) = session.undo_label().filter(|label| !label.is_empty()) {
        cells.push((format!("deshacer {label}"), false));
    }
    cells.push(("cm".to_owned(), false));
    cells
}

/// What the bar says with nothing on the mat to measure.
fn empty() -> Vec<(String, bool)> {
    vec![("mesa vacía".to_owned(), false), ("cm".to_owned(), false)]
}

/// The whole product: how many pieces it holds, which of them is broken, and
/// what a press does next — open a piece, or with the sewing tool in hand pick
/// the side the seam is waiting for.
fn product(draft: &Draft, state: &State) -> Vec<(String, bool)> {
    let doc = draft.doc();
    let count = match doc.pieces.len() {
        0 => "sin piezas".to_owned(),
        1 => "1 pieza".to_owned(),
        many => format!("{many} piezas"),
    };
    let mut cells = vec![("todas las piezas".to_owned(), false), (count, false)];
    for (key, held) in doc.pieces.iter() {
        if !draft.defects(key).is_empty() {
            cells.push((format!("{}: contorno con defectos", held.name), true));
        }
    }
    let next = match (state.tool, &state.gesture) {
        (Tool::Sew, Gesture::Sewing(_)) => {
            "coser: pulsa el segundo tramo · Mayús alarga el lado · Esc suelta"
        }
        (Tool::Sew, _) if state.selection.seam().is_some() => {
            "coser: pulsa el primer tramo · Mayús ajusta un lado de la costura elegida"
        }
        (Tool::Sew, _) => "coser: pulsa el primer tramo",
        _ => "doble clic abre una pieza",
    };
    if !doc.pieces.is_empty() {
        cells.push((next.to_owned(), false));
    }
    cells
}

/// One piece: its name, its nodes, its side seam, and whether its contour
/// still holds.
fn one(draft: &Draft, piece: PieceKey) -> Vec<(String, bool)> {
    let name = draft
        .doc()
        .pieces
        .get(piece)
        .map_or_else(String::new, |held| held.name.clone());
    let mut cells = vec![
        (name, false),
        (format!("{} puntos", draft.points_cm(piece).len()), false),
        (side_cell(draft, piece), false),
    ];
    if !draft.defects(piece).is_empty() {
        cells.push(("contorno con defectos".to_owned(), true));
    }
    cells
}

/// The side seam, walked along the contour between the two nodes that name it.
pub(super) fn side_cell(draft: &Draft, piece: PieceKey) -> String {
    let doc = draft.doc();
    let ends = (
        doc.shows_label(piece, SIDE[0]),
        doc.shows_label(piece, SIDE[1]),
    );
    match ends {
        (Some(from), Some(to)) => {
            format!("lateral {:.1} cm", draft.run_length_cm(piece, from, to))
        }
        _ => format!("perímetro {:.1} cm", draft.perimeter_cm(piece)),
    }
}
