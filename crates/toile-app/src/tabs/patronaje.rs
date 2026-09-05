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
use self::state::Tool;
use self::wire::Verb;
use crate::tabs::{Workspace, left_panel, right_panel};

/// The two nodes a base block names for the side seam, so the bar can measure
/// it instead of quoting it. A piece that names neither reports its perimeter.
const SIDE: [&str; 2] = ["cintura_lat", "bajo_lat"];

pub fn show(ui: &mut egui::Ui, w: &mut Workspace<'_>) {
    let theme = w.theme;
    let piece = w.session.piece();
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
    let state = &mut *w.patronaje;
    let mut verbs = Vec::new();
    verbs.extend(left_panel(ui, theme, |ui| {
        let plea = tree::product(ui, theme, draft, piece);
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
        inspector::show(ui, theme, draft, piece, state)
    });
    // One field confirmed is one entry of its own, under its own name: an edit
    // from a panel never folds into whatever gesture the mat left open.
    if let Some((label, command)) = asked.filter(|_| !asking) {
        verbs.push(Verb::Begin(label));
        verbs.push(Verb::Edit(Box::new(command)));
        verbs.push(Verb::End);
    }
    verbs.extend(canvas::show(ui, theme, draft, piece, state));
    if apply(w.session, verbs, &mut w.patronaje.refused) {
        // The bars are drawn before the tabs, so what this run has to say
        // reaches the status bar on the frame after it. Nothing else asks for
        // that frame: a refusal sends nothing to the sim, so the viewer is
        // asleep and would sit on a stale bar until the pointer moved again.
        ui.ctx().request_repaint();
    }
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
    let (Some(draft), Some(piece)) = (session.draft(), session.piece()) else {
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
mod tests {
    use toile_engine::draft::{
        Axis, Binding, Command, Doc, MeasureSet, Piece, Point, Winding, block,
    };

    use super::*;

    /// The two edits the table takes in its stride say nothing: a shape edit
    /// re-derives, a topology edit goes to the mesher. The one the document
    /// refuses has to reach the status bar, because the drawing goes on being
    /// edited either way.
    #[test]
    fn an_edit_the_session_refuses_is_said_in_the_status_bar() {
        let mut session = Session::from_doc(block::trouser_front()).expect("the block drapes");
        let piece = session.piece().expect("the session has a document");
        let node = session
            .draft()
            .expect("the session has a document")
            .points_cm(piece)[0]
            .0;
        let mut state = State::default();

        let moved = Verb::Edit(Box::new(Command::SetBinding {
            point: node,
            axis: Axis::X,
            to: Binding::literal(3.0),
        }));
        assert!(!apply(&mut session, vec![moved], &mut state.refused));
        assert_eq!(
            state.refused, None,
            "a shape edit re-drapes and says nothing"
        );

        let sampled = Verb::Edit(Box::new(Command::SetSamples {
            piece,
            node,
            to: 24,
        }));
        assert!(!apply(&mut session, vec![sampled], &mut state.refused));
        assert!(session.remeshing(), "the rebuild is out with the mesher");

        // A sample count no tract may take: the document refuses it, and the
        // table has to say so.
        let refused = Verb::Edit(Box::new(Command::SetSamples {
            piece,
            node,
            to: 4096,
        }));
        assert!(apply(&mut session, vec![refused], &mut state.refused));
        let cells = status(&session, &state);
        assert!(
            cells
                .iter()
                .any(|(cell, alert)| cell.starts_with("rechazado") && *alert),
            "{cells:?}"
        );
    }

    #[test]
    fn the_side_seam_cell_is_measured_not_quoted() {
        let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
        let piece = draft
            .doc()
            .piece_named(block::FRONT)
            .expect("the block draws one piece");
        // Measured along the flattening: the hip is a curve, so the seam is a
        // millimetre longer than the chords through its nodes.
        assert_eq!(side_cell(&draft, piece), "lateral 104.6 cm");
    }

    #[test]
    fn a_piece_that_does_not_name_its_side_reports_its_perimeter() {
        let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
        let corners = [[0.0, 0.0], [10.0, 0.0], [10.0, 20.0], [0.0, 20.0]];
        let points: Vec<_> = corners
            .into_iter()
            .map(|[x, y]| doc.points.insert(Point::at(x, y)))
            .collect();
        let piece = doc
            .pieces
            .insert(Piece::polygon("Cuadro", points, Winding::Cw));
        let draft = Draft::from_doc(doc).expect("a square resolves");
        assert_eq!(side_cell(&draft, piece), "perímetro 60.0 cm");
    }
}
