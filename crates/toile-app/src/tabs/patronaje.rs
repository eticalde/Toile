mod arrange;
mod canvas;
mod caption;
mod curve;
mod dimension;
mod empty;
mod gesture;
mod input;
mod inspector;
mod layout;
mod marks;
mod modal;
mod overview;
mod paper;
mod pick;
mod precision;
mod report;
mod ruler;
mod snap;
mod state;
mod tools;
mod tract;
mod tree;
mod view;
mod wire;

use eframe::egui;
pub use report::status;
pub use state::State;
use toile_engine::draft::{Command, Draft, PieceKey};
use toile_engine::session::Session;

use self::gesture::Gesture;
use self::state::{Scope, Tool};
use self::wire::Verb;
use crate::file::Action;
use crate::tabs::{Workspace, left_panel, right_panel};
use crate::theme::Theme;

pub fn show(ui: &mut egui::Ui, w: &mut Workspace<'_>) {
    table(ui, w.theme, w.session, w.patronaje);
}

/// The whole tab, out of what the workspace hands it: the product tree and
/// the tools, the inspector, and the mat between them.
fn table(ui: &mut egui::Ui, theme: &Theme, session: &mut Session, patronaje: &mut State) {
    // A question waiting on the mat owns the open entry until it is answered.
    // The tiles that would move the stack under it go dead, and so does every
    // edit a panel offers: an entry belongs to the gesture that opened it.
    let asking = patronaje.ask.is_some();
    let ready = if asking {
        [false, false]
    } else {
        [session.can_undo(), session.can_redo()]
    };
    let draft = session.draft();
    // The piece in front this frame, and the pieces there were before an
    // edit: what an edit adds or takes away is what the mat follows.
    let active = active_piece(draft, patronaje.active, session.piece());
    let before = draft.map(|d| d.doc().piece_keys()).unwrap_or_default();
    patronaje.active = active;
    let state = &mut *patronaje;
    let mut verbs = Vec::new();
    let mat = tree::Mat {
        scope: state.scope,
        drawing: matches!(state.gesture, Gesture::Drawing { .. }),
        live: !asking,
    };
    verbs.extend(left_panel(ui, theme, |ui| {
        let plea = tree::product(ui, theme, draft, active, &mut state.renaming, mat);
        tools::grid(ui, theme, state);
        let mut asked: Vec<Verb> = tools::history(ui, theme, ready).into_iter().collect();
        if let Some(plea) = plea.filter(|_| !asking) {
            // An open drawing has nothing to send the sim, so nothing else
            // will ask for the frame on which the bar catches up with the
            // press.
            if plea == tree::Plea::Draw {
                ui.ctx().request_repaint();
            }
            asked.extend(plead(state, plea, draft.is_some()));
        }
        asked
    }));
    let asked = right_panel(ui, theme, |ui| {
        inspector::show(ui, theme, draft, active, state)
    });
    // The panel brackets its own entries: a field confirmed is one of its own
    // under its own name, and a rail dragged holds one open across frames. A
    // question on the mat owns the stack, so while one waits none of them is
    // played at all.
    if !asking {
        verbs.extend(asked);
    }
    verbs.extend(canvas::show(ui, theme, draft, active, state));
    let said = apply(session, verbs, &mut patronaje.refused);
    let moved = follow(session, patronaje, &before);
    if said || moved {
        // The bars are drawn before the tabs, so what this run has to say
        // reaches the status bar on the frame after it. Nothing else asks for
        // that frame: a refusal sends nothing to the sim, so the viewer is
        // asleep and would sit on a stale bar until the pointer moved again.
        ui.ctx().request_repaint();
    }
}

/// The piece in front, in order of preference: the one chosen while it still
/// exists, then the one draping, then the first the product holds.
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

/// What the product tree's plea does to the tab, and the edits it asks for.
///
/// A new drawing and the whole product wait for the mat to be free of every
/// gesture but a drawing, which they walk away from: any other gesture may
/// hold the undo stack open, and a view left behind would leave that entry
/// open with it.
fn plead(state: &mut State, plea: tree::Plea, has_document: bool) -> Vec<Verb> {
    let free = matches!(state.gesture, Gesture::Idle | Gesture::Drawing { .. });
    match plea {
        // A drawing already in progress starts over: the row was pressed to
        // start one.
        tree::Plea::Draw if free => begin_piece(state, has_document),
        tree::Plea::Overview if free => state.overview(),
        tree::Plea::Focus(key) => state.open(key),
        tree::Plea::Rename(piece, to) => {
            return entry("renombrar pieza", Command::RenamePiece { piece, to });
        }
        tree::Plea::Remove(piece) => {
            return entry("borrar pieza", Command::RemovePiece { piece });
        }
        tree::Plea::Draw | tree::Plea::Overview => {}
    }
    Vec::new()
}

/// One edit from a panel, as an undo entry of its own under its own name.
fn entry(label: &'static str, command: Command) -> Vec<Verb> {
    vec![Verb::Begin(label), Verb::Edit(Box::new(command)), Verb::End]
}

/// What "+ Pieza" does, which turns on whether there is a product to add the
/// piece to.
///
/// With a document on the table it opens the drawing gesture, so the very next
/// click on the mat places the first vertex — the one deliberate way to begin a
/// piece. A contour is drawn in the document's own coordinates, which only a
/// piece's detail shows, so from the whole product the mat first turns to the
/// detail of the piece in front, whose nodes the new contour may catch on. With
/// no document it asks the application for a product instead, down the channel
/// the empty mat's own splash asks along: a piece cannot exist outside a
/// product, so naming one is what the press really wants, and it is one click
/// away instead of none.
fn begin_piece(state: &mut State, has_document: bool) {
    if !has_document {
        state.asked = Some(Action::New);
        return;
    }
    // A drawing started over keeps the scope the first one was begun from.
    let back_to = match state.gesture {
        Gesture::Drawing { back_to, .. } => back_to,
        _ => state.scope,
    };
    if state.scope == Scope::Product {
        state.scope = Scope::Piece;
        state.frame = true;
    }
    state.tool = Tool::Select;
    state.gesture = Gesture::Drawing {
        pending: Vec::new(),
        rubber: [0.0, 0.0],
        back_to,
    };
}

/// Keeps the mat on a piece that exists, and says whether the pieces on the
/// table changed at all — an addition to follow, or a removal to redraw for.
///
/// A piece just drawn is opened, so the mat follows the hand onto what it drew.
/// On the whole product a piece that comes back, as an undone removal brings
/// it, is only put in front. A detail whose piece has gone falls back to the
/// whole product rather than to some other piece nobody asked for.
fn follow(session: &Session, state: &mut State, before: &[PieceKey]) -> bool {
    let after = session
        .draft()
        .map(|d| d.doc().piece_keys())
        .unwrap_or_default();
    if let Some(&fresh) = after.iter().find(|key| !before.contains(key)) {
        match state.scope {
            Scope::Piece => state.open(fresh),
            Scope::Product => state.active = Some(fresh),
        }
    } else if state.scope == Scope::Piece && state.active.is_some_and(|key| !after.contains(&key)) {
        state.overview();
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

#[cfg(test)]
mod tests;
