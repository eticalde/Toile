use eframe::egui::epaint::Shape;
use eframe::egui::{Id, Pos2, Rect};
use toile_engine::draft::{PieceKey, block};

use super::super::inspector::write::id_of;
use super::super::state::{Cut, Field};
use super::bench::front_and_back;
use super::studio::Studio;

mod shows;
mod wrote;

/// The shipped trousers with the owner's own DELANTERO written onto the front:
/// the letter A, two of them, a margin of 1.5 cm, the two lines of its label.
///
/// Read off `baggy-jeans.sm2d`, and the row this panel exists for: a label
/// that says «cortar 2 espejadas» beside a count that says 2.
pub(super) fn delantero() -> (Studio, PieceKey) {
    stamped("A", 2, Some(1.5), &["DELANTERO", "cortar 2 espejadas"])
}

/// The same table over TIRA PASACINTOS, which is one of the owner's two net
/// pieces: a strip folded in thirds, with no allowance written for it at all.
pub(super) fn tira() -> (Studio, PieceKey) {
    stamped(
        "F",
        1,
        None,
        &["TIRA PASACINTOS", "cortar 1 - doblar en tercios"],
    )
}

/// The shipped front wearing one of the owner's rows, on the whole tab, with
/// nothing on the piece chosen.
fn stamped(letter: &str, quantity: u32, width: Option<f64>, labels: &[&str]) -> (Studio, PieceKey) {
    let mut doc = block::trousers();
    let (front, _) = front_and_back(&doc);
    let held = doc.pieces.get_mut(front).expect("the block draws a front");
    held.letter = Some(letter.to_owned());
    held.quantity = quantity;
    held.seam_allowance = width;
    held.labels = labels.iter().map(|&line| line.to_owned()).collect();
    let mut studio = Studio::new(doc);
    studio.frame(Vec::new());
    (studio, front)
}

/// Every line of text the last frame put where a person could read it.
///
/// Clipped out counts as not said: the panel scrolls, so a row laid out below
/// the glass is painted into shapes nobody ever sees, and a test that read
/// those would pass on a panel whose boxes are all off the bottom.
pub(super) fn words(studio: &Studio) -> Vec<String> {
    words_at(studio).into_iter().map(|(_, text)| text).collect()
}

/// The same lines, each with where the frame put it.
fn words_at(studio: &Studio) -> Vec<(Pos2, String)> {
    fn open(shape: &Shape, clip: Rect, out: &mut Vec<(Pos2, String)>) {
        match shape {
            Shape::Vec(inner) => inner.iter().for_each(|it| open(it, clip, out)),
            Shape::Text(text) if clip.contains(text.pos) => {
                out.push((text.pos, text.galley.text().to_owned()));
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &studio.shapes {
        open(&clipped.shape, clipped.clip_rect, &mut out);
    }
    out
}

/// Whether the last frame wrote `want` as a line of its own, anywhere a person
/// could read it.
pub(super) fn says(studio: &Studio, want: &str) -> bool {
    words(studio).iter().any(|line| line == want)
}

/// Where the box of one of the piece's cut rows was drawn.
pub(super) fn box_of(piece: PieceKey, of: Cut) -> Id {
    id_of(&Field::Cut(piece, of))
}

/// Whether the panel drew that box at all.
pub(super) fn drew(studio: &Studio, id: Id) -> bool {
    studio.ctx.read_response(id).is_some()
}

/// What the last frame painted inside one box, where a person would read it.
///
/// The value in its own box and not a line that happens to read the same: a
/// count of two is one character, and the panel paints a two elsewhere for
/// every other reason there is.
pub(super) fn inside(studio: &Studio, id: Id) -> Option<String> {
    let rect = studio.ctx.read_response(id)?.rect;
    words_at(studio)
        .into_iter()
        .find(|(at, _)| rect.contains(*at))
        .map(|(_, text)| text)
}

/// How many entries the product's history would take back.
pub(super) fn entries(studio: &Studio) -> usize {
    studio
        .session
        .draft()
        .expect("a product is open")
        .undo_depth()
}

/// What a piece says about being cut out, as the document holds it.
pub(super) struct Said {
    pub(super) letter: Option<String>,
    pub(super) quantity: u32,
    pub(super) allowance: Option<f64>,
    pub(super) labels: Vec<String>,
}

/// What the document says about cutting the piece out right now.
pub(super) fn held(studio: &Studio, piece: PieceKey) -> Said {
    let piece = studio.doc().pieces.get(piece).expect("the piece is live");
    Said {
        letter: piece.letter.clone(),
        quantity: piece.quantity,
        allowance: piece.seam_allowance,
        labels: piece.labels.clone(),
    }
}
