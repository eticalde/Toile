use toile_engine::draft::{LineKey, PieceKey, SeamKey};
use toile_engine::session::Session;

use super::state::{Scope, Selection, State};

/// Keeps the mat on a piece that exists, and says whether the pieces on the
/// table changed at all — an addition to follow, or a removal to redraw for.
///
/// A piece just drawn is opened, so the mat follows the hand onto what it drew.
/// On the whole product a piece that comes back, as an undone removal brings
/// it, is only put in front. A detail whose piece has gone falls back to the
/// whole product rather than to some other piece nobody asked for.
pub fn pieces(session: &Session, state: &mut State, before: &[PieceKey]) -> bool {
    let after = session
        .draft()
        .map(|d| d.doc().piece_keys())
        .unwrap_or_default();
    if let Some(&fresh) = after.iter().find(|key| !before.contains(key)) {
        match state.scope {
            Scope::Piece => state.open(fresh),
            Scope::Product => state.front(fresh),
        }
    } else if state.scope == Scope::Piece && state.active.is_some_and(|key| !after.contains(&key)) {
        state.overview();
    }
    after != before
}

/// The seams the product holds, in key order.
pub fn seam_keys(session: &Session) -> Vec<SeamKey> {
    session
        .draft()
        .map(|d| d.doc().seams.keys().collect())
        .unwrap_or_default()
}

/// Keeps the chosen seam one that exists: a seam that has just arrived, sewn
/// or brought back by an undo, is the one chosen, so the inspector opens on
/// its lengths and on the press that turns it over; a chosen seam that has
/// gone leaves nothing chosen.
pub fn seams(session: &Session, state: &mut State, before: &[SeamKey]) {
    let after = seam_keys(session);
    if state.scope != Scope::Product {
        return;
    }
    if let Some(&fresh) = after.iter().find(|key| !before.contains(key)) {
        state.choose(Selection::Seam(fresh));
    } else if state
        .selection
        .seam()
        .is_some_and(|key| !after.contains(&key))
    {
        state.choose(Selection::None);
    }
}

/// The internal lines the product holds, in key order.
pub fn line_keys(session: &Session) -> Vec<LineKey> {
    session
        .draft()
        .map(|d| d.doc().lines.keys().collect())
        .unwrap_or_default()
}

/// Keeps the chosen internal line one that exists: a line just traced, or one
/// an undo has brought back, is the one chosen, so the inspector opens on its
/// kind; a chosen line that has gone leaves nothing chosen.
///
/// Only over a piece on its own, which is where a line is drawn and edited. On
/// the whole product the lines are there to be seen and nothing else.
pub fn lines(session: &Session, state: &mut State, before: &[LineKey]) {
    let after = line_keys(session);
    if state.scope != Scope::Piece {
        return;
    }
    if let Some(&fresh) = after.iter().find(|key| !before.contains(key)) {
        state.choose(Selection::Line(fresh));
    } else if state
        .selection
        .line()
        .is_some_and(|key| !after.contains(&key))
    {
        state.choose(Selection::None);
    }
}
