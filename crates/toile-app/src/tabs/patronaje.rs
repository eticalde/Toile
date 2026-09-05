mod canvas;
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
mod tree;
mod view;
mod wire;

use eframe::egui;
pub use state::State;
use toile_engine::draft::{Draft, PieceKey};
use toile_engine::session::Session;

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
        tree::product(ui, theme, draft, piece);
        tools::grid(ui, theme, state);
        tools::history(ui, theme, ready)
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
    apply(w.session, verbs);
}

/// Plays what the panels asked for, in the order they asked for it.
///
/// A refused edit leaves the document exactly as it was, and the panels go on
/// drawing it: the table never shows a state it cannot resolve.
fn apply(session: &mut Session, verbs: Vec<Verb>) {
    for verb in verbs {
        match verb {
            Verb::Begin(label) => session.begin_gesture(label),
            Verb::Edit(command) => {
                let _ = session.edit(*command);
            }
            Verb::End => session.end_gesture(),
            Verb::Undo => {
                let _ = session.undo();
            }
            Verb::Cancel => {
                let _ = session.cancel_gesture();
            }
            Verb::Redo => {
                let _ = session.redo();
            }
        }
    }
}

/// The cells of the status bar, measured off the document on the table.
pub fn status(session: &Session) -> Vec<String> {
    let (Some(draft), Some(piece)) = (session.draft(), session.piece()) else {
        return vec!["mesa vacía".to_owned(), "cm".to_owned()];
    };
    let name = draft
        .doc()
        .pieces
        .get(piece)
        .map_or_else(String::new, |held| held.name.clone());
    let mut cells = vec![
        name,
        format!("{} puntos", draft.points_cm(piece).len()),
        side_cell(draft, piece),
    ];
    if !draft.defects(piece).is_empty() {
        cells.push("contorno con defectos".to_owned());
    }
    if let Some(label) = session.undo_label().filter(|label| !label.is_empty()) {
        cells.push(format!("deshacer {label}"));
    }
    cells.push("cm".to_owned());
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
    use toile_engine::draft::{Doc, MeasureSet, Piece, Point, Winding, block};

    use super::*;

    #[test]
    fn the_side_seam_cell_is_measured_not_quoted() {
        let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
        let piece = draft
            .doc()
            .piece_named(block::FRONT)
            .expect("the block draws one piece");
        assert_eq!(side_cell(&draft, piece), "lateral 104.5 cm");
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
