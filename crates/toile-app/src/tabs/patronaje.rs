mod canvas;
mod curve;
mod dimension;
mod empty;
mod gesture;
mod input;
mod inspector;
mod marks;
mod modal;
mod paper;
mod pick;
mod precision;
mod ruler;
mod snap;
mod state;
mod tools;
mod tract;
mod tree;
mod view;
mod wire;

use eframe::egui;
pub use state::State;
use toile_engine::draft::{Command, Draft, PieceKey};
use toile_engine::session::Session;

use self::gesture::Gesture;
use self::state::{Selection, Tool};
use self::wire::Verb;
use crate::tabs::{Workspace, left_panel, right_panel};

/// The two nodes a base block names for the side seam, so the bar can measure
/// it instead of quoting it. A piece that names neither reports its perimeter.
const SIDE: [&str; 2] = ["cintura_lat", "bajo_lat"];

pub fn show(ui: &mut egui::Ui, w: &mut Workspace<'_>) {
    let theme = w.theme;
    // A question waiting on the mat owns the open entry until it is answered.
    // The tiles that would move the stack under it go dead, and so does every
    // edit a panel offers: an entry belongs to the gesture that opened it.
    let asking = w.patronaje.ask.is_some();
    let ready = if asking {
        [false, false]
    } else {
        [w.session.can_undo(), w.session.can_redo()]
    };
    let draft = w.session.draft();
    // The piece the mat draws this frame, and the pieces there were before an
    // edit: a piece drawn during it becomes the active one afterwards.
    let active = active_piece(draft, w.patronaje.active, w.session.piece());
    let before = draft.map(|d| d.doc().piece_keys()).unwrap_or_default();
    w.patronaje.active = active;
    let state = &mut *w.patronaje;
    let mut verbs = Vec::new();
    verbs.extend(left_panel(ui, theme, |ui| {
        let plea = tree::product(ui, theme, draft, active);
        tools::grid(ui, theme, state);
        let mut asked: Vec<Verb> = tools::history(ui, theme, ready).into_iter().collect();
        match plea.filter(|_| !asking) {
            // "+ Pieza" puts the Line tool in hand with the drawing already
            // begun, so the very next click on the mat places a vertex. A
            // drawing already in progress starts over: the row was pressed
            // to start one.
            Some(tree::Plea::Draw)
                if matches!(state.gesture, Gesture::Idle | Gesture::Drawing { .. }) =>
            {
                state.tool = Tool::Line;
                state.gesture = Gesture::Drawing {
                    pending: Vec::new(),
                    rubber: [0.0, 0.0],
                };
            }
            Some(tree::Plea::Focus(key)) => focus(state, key),
            Some(tree::Plea::Remove(key)) => {
                asked.push(Verb::Begin("borrar pieza"));
                asked.push(Verb::Edit(Box::new(Command::RemovePiece { piece: key })));
                asked.push(Verb::End);
            }
            Some(tree::Plea::Draw) | None => {}
        }
        asked
    }));
    let asked = right_panel(ui, theme, |ui| {
        inspector::show(ui, theme, draft, active, state)
    });
    // One field confirmed is one entry of its own, under its own name: an edit
    // from a panel never folds into whatever gesture the mat left open.
    if let Some((label, command)) = asked.filter(|_| !asking) {
        verbs.push(Verb::Begin(label));
        verbs.push(Verb::Edit(Box::new(command)));
        verbs.push(Verb::End);
    }
    verbs.extend(canvas::show(ui, theme, draft, active, state));
    let said = apply(w.session, verbs, &mut w.patronaje.refused);
    // A piece just drawn becomes the active one, so the mat follows the hand
    // onto what it drew rather than staying on what was there before.
    let moved = adopt_new(w, &before);
    if said || moved {
        // The bars are drawn before the tabs, so what this run has to say
        // reaches the status bar on the frame after it. Nothing else asks for
        // that frame: a refusal sends nothing to the sim, so the viewer is
        // asleep and would sit on a stale bar until the pointer moved again.
        ui.ctx().request_repaint();
    }
}

/// The piece the mat draws, in order of preference: the one chosen while it
/// still exists, then the one draping, then the first the product holds.
///
/// `None` only for a product with no pieces at all — a blank mat waiting for
/// its first piece to be drawn.
fn active_piece(
    draft: Option<&Draft>,
    chosen: Option<PieceKey>,
    draping: Option<PieceKey>,
) -> Option<PieceKey> {
    let doc = draft?.doc();
    let has = |key: PieceKey| doc.pieces.get(key).is_some();
    chosen
        .filter(|&key| has(key))
        .or(draping.filter(|&key| has(key)))
        .or_else(|| doc.piece_keys().first().copied())
}

/// Brings a piece to the front: the mat draws it, nothing of the last piece
/// stays chosen, and the view fits it on the next frame.
fn focus(state: &mut State, piece: PieceKey) {
    state.active = Some(piece);
    state.selection = Selection::None;
    state.frame = true;
}

/// Makes a piece just added the active one, and says whether the pieces on the
/// table changed at all — an addition to follow, or a removal to redraw for.
fn adopt_new(w: &mut Workspace<'_>, before: &[PieceKey]) -> bool {
    let after = w
        .session
        .draft()
        .map(|d| d.doc().piece_keys())
        .unwrap_or_default();
    if let Some(&fresh) = after.iter().find(|key| !before.contains(key)) {
        focus(w.patronaje, fresh);
    }
    after != before
}

/// Plays what the panels asked for, in the order they asked for it, and
/// leaves in `said` whatever the session refused.
///
/// A refused edit leaves the document exactly as it was, and the panels go on
/// drawing it: the table never shows a state it cannot resolve. What must not
/// be swallowed is the refusal itself — the drawing goes on being edited
/// either way, and a refusal nobody says is the piece on the stand quietly
/// parting from the table.
///
/// Answers whether what there is to say changed, which is what asks for the
/// frame that says it.
fn apply(session: &mut Session, verbs: Vec<Verb>, said: &mut Option<String>) -> bool {
    let mut refused = None;
    let mut played = false;
    for verb in verbs {
        let answer = match verb {
            Verb::Begin(label) => {
                session.begin_gesture(label);
                continue;
            }
            Verb::End => {
                session.end_gesture();
                continue;
            }
            Verb::Edit(command) => session.edit(*command),
            Verb::Undo => session.undo(),
            Verb::Cancel => session.cancel_gesture(),
            Verb::Redo => session.redo(),
        };
        played = true;
        if let Err(why) = answer
            && refused.is_none()
        {
            refused = Some(why.to_string());
        }
    }
    if !played || *said == refused {
        return false;
    }
    *said = refused;
    true
}

/// The cells of the status bar, measured off the document on the table.
///
/// Each cell says whether it is an alert: a refused edit and a broken contour
/// are painted to be seen, and everything else stays quiet.
pub fn status(session: &Session, state: &State) -> Vec<(String, bool)> {
    let draft = session.draft();
    let piece = active_piece(draft, state.active, session.piece());
    let (Some(draft), Some(piece)) = (draft, piece) else {
        return vec![("mesa vacía".to_owned(), false), ("cm".to_owned(), false)];
    };
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
    if let Some(why) = state.refused.as_deref() {
        cells.push((format!("rechazado: {why}"), true));
    }
    if let Some(label) = session.undo_label().filter(|label| !label.is_empty()) {
        cells.push((format!("deshacer {label}"), false));
    }
    cells.push(("cm".to_owned(), false));
    cells
}

/// The side seam, walked along the contour between the two nodes that name it.
fn side_cell(draft: &Draft, piece: PieceKey) -> String {
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

#[cfg(test)]
mod tests;
