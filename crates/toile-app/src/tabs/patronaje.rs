mod canvas;
mod input;
mod inspector;
mod paper;
mod ruler;
mod state;
mod tools;
mod tree;
mod view;

use eframe::egui;
pub use state::State;
use toile_engine::draft::{Draft, PieceKey};
use toile_engine::session::Session;

use crate::tabs::{Workspace, left_panel, right_panel};

/// The two nodes a base block names for the side seam, so the bar can measure
/// it instead of quoting it. A piece that names neither reports its perimeter.
const SIDE: [&str; 2] = ["cintura_lat", "bajo_lat"];

pub fn show(ui: &mut egui::Ui, w: &mut Workspace<'_>) {
    let theme = w.theme;
    let piece = w.session.piece();
    let draft = w.session.draft();
    let state = &mut *w.patronaje;
    left_panel(ui, theme, |ui| {
        tree::product(ui, theme, draft, piece, state);
        tools::grid(ui, theme);
    });
    let command = right_panel(ui, theme, |ui| {
        inspector::show(ui, theme, draft, piece, state)
    });
    canvas::show(ui, theme, draft, piece, state);
    if let Some(command) = command {
        // A refused edit leaves the document exactly as it was, and the panels
        // go on drawing it: the table never shows a state it cannot resolve.
        let _ = w.session.edit(command);
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
