use eframe::egui::epaint::{ColorMode, Shape};
use eframe::egui::{Event, Key, Modifiers, Pos2, vec2};
use toile_engine::draft::{Doc, EdgeRange, PieceKey, Seam, SeamKey, SeamOrientation, block};

use super::super::gesture::Gesture;
use super::super::inspector::seams::{flip_id, row_id, unpick_id};
use super::super::state::{Scope, Selection, Tool};
use super::super::{layout, sew};
use super::bench::front_and_back;
use super::studio::{Studio, painted};

/// The shipped trousers with their two seams unpicked: a front and a back on
/// the table and nothing between them.
pub(super) fn unsewn() -> Doc {
    let mut doc = block::trousers();
    let keys: Vec<SeamKey> = doc.seams.keys().collect();
    for key in keys {
        doc.seams.remove(key).expect("the key is live");
    }
    doc
}

/// A place on the tract leaving a named node, about half way along the line
/// the whole product draws for it right now, on the glass.
pub(super) fn on_tract(studio: &Studio, piece: PieceKey, from: &str) -> Pos2 {
    let draft = studio.session.draft().expect("a product is open");
    let node = draft.doc().shows_label(piece, from).expect("a named node");
    let spread = sew::spread(draft, &layout::of(draft));
    let on = spread.iter().find(|it| it.piece == piece).expect("laid");
    let tract = on.tracts.iter().find(|it| it.node == node).expect("drawn");
    let k = (tract.line.len() - 1) / 2;
    let (head, tail) = (tract.line[k], tract.line[k + 1]);
    let middle = [0, 1].map(|axis| f64::midpoint(head[axis], tail[axis]));
    studio.state.view.to_screen(middle)
}

/// Whether the last frame wrote `what` anywhere on the tab.
pub(super) fn says(studio: &Studio, what: &str) -> bool {
    painted(studio)
        .iter()
        .any(|shape| matches!(shape, Shape::Text(text) if text.galley.text().contains(what)))
}

/// Where the last frame wrote exactly `what`.
fn written(studio: &Studio, what: &str) -> Pos2 {
    painted(studio)
        .iter()
        .find_map(|shape| match shape {
            Shape::Text(text) if text.galley.text() == what => Some(text.pos),
            _ => None,
        })
        .expect("the tab wrote it")
}

/// The one seam the product holds.
pub(super) fn only(studio: &Studio) -> (SeamKey, Seam) {
    let doc = studio.doc();
    assert_eq!(doc.seams.len(), 1, "one seam, no more");
    let (key, held) = doc.seams.iter().next().expect("the arena holds one");
    (key, *held)
}

/// The front's knee-to-hem side sewn to the back's, by the tool's two presses.
fn sewn() -> Studio {
    let mut studio = Studio::new(unsewn());
    studio.frame(Vec::new());
    studio.key(Key::S, Modifiers::NONE);
    assert_eq!(studio.state.tool, Tool::Sew, "S takes the sewing tool");
    let (front, back) = front_and_back(studio.doc());
    let first = on_tract(&studio, front, "rodilla_lat");
    studio.click(first);
    assert!(matches!(studio.state.gesture, Gesture::Sewing(_)));
    assert_eq!(studio.doc().seams.len(), 0, "one side is not a seam");
    let second = on_tract(&studio, back, "rodilla_lat_tras");
    studio.click(second);
    studio.frame(Vec::new());
    studio
}

#[test]
fn two_presses_with_the_sewing_tool_sew_the_front_to_the_back() {
    let studio = sewn();
    let (front, back) = front_and_back(studio.doc());
    let (key, held) = only(&studio);
    let doc = studio.doc();
    let node = |piece, label| doc.shows_label(piece, label).expect("a named node");
    let a = EdgeRange::between(front, node(front, "rodilla_lat"), node(front, "bajo_lat"));
    let tail = node(back, "bajo_lat_tras");
    let b = EdgeRange::between(back, node(back, "rodilla_lat_tras"), tail);
    // Both sides run knee to hem down the mat, so knee meets knee.
    assert_eq!(held, Seam::plain(a, b, SeamOrientation::Aligned));
    assert_eq!(studio.session.undo_label(), Some("coser"));
    assert_eq!(studio.state.gesture, Gesture::Idle);
    assert_eq!(
        studio.state.selection,
        Selection::Seam(key),
        "and is chosen"
    );
    assert_eq!(studio.state.scope, Scope::Product);
    assert!(says(&studio, "COSTURA 1"), "the inspector opens on it");
    assert!(says(&studio, "A · Delantero · rodilla_lat → bajo_lat"));
    let thread = ColorMode::Solid(studio.theme.seam);
    let drawn = painted(&studio)
        .iter()
        .filter(|shape| matches!(shape, Shape::Path(path) if path.stroke.color == thread))
        .count();
    assert_eq!(drawn, 2, "one thread along each side, in the seam's colour");
}

#[test]
fn a_seam_sewn_on_the_table_reaches_the_cloth() {
    let mut studio = sewn();
    studio.session.wait_for_remesh().expect("both pieces mesh");
    assert_eq!(studio.session.seam_faults(), &[], "the engine pairs it");
    assert!(
        !studio.session.sewn_pairs().is_empty(),
        "the solver holds the front to the back along it"
    );
}

#[test]
fn turning_a_seam_over_is_one_entry_and_keeps_its_key() {
    let mut studio = sewn();
    let (key, _) = only(&studio);
    studio.click(studio.centre(flip_id(key)));
    studio.frame(Vec::new());
    let (again, held) = only(&studio);
    assert_eq!(again, key, "the same seam, under its own key");
    assert_eq!(held.orientation, SeamOrientation::Opposed);
    assert_eq!(
        studio.session.undo_label(),
        Some("invertir el sentido de la costura")
    );
    assert_eq!(studio.state.selection, Selection::Seam(key));
    assert!(says(&studio, "contrario"));

    studio.session.undo().expect("the entry steps back");
    assert_eq!(only(&studio).1.orientation, SeamOrientation::Aligned);
    assert_eq!(studio.session.undo_label(), Some("coser"), "one entry each");
}

/// The rows of the section stack, and its two presses stand side by side: a
/// control drawn over another is a control nobody can aim at.
#[test]
fn the_seam_row_and_its_two_presses_do_not_land_on_one_another() {
    let studio = sewn();
    let (key, _) = only(&studio);
    let rect = |id| studio.ctx.read_response(id).expect("the tab drew it").rect;
    let (row, flip, unpick) = (rect(row_id(key)), rect(flip_id(key)), rect(unpick_id(key)));
    assert!(row.bottom() < flip.top(), "{row:?} {flip:?}");
    assert!(flip.right() < unpick.left(), "{flip:?} {unpick:?}");
    assert!(unpick.right() < 1320.0, "inside the panel: {unpick:?}");
}

#[test]
fn undo_and_redo_take_the_seam_away_and_bring_the_same_one_back() {
    let mut studio = sewn();
    let (key, held) = only(&studio);
    let mat = on_tract(&studio, front_and_back(studio.doc()).0, "rodilla_lat") + vec2(60.0, 0.0);
    studio.frame(vec![Event::PointerMoved(mat)]);
    studio.key(Key::Z, Modifiers::COMMAND);
    assert_eq!(studio.doc().seams.len(), 0, "undone");
    assert_eq!(studio.state.selection, Selection::None);
    assert!(!studio.session.can_undo(), "sewing was the only entry");

    studio.key(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(only(&studio), (key, held), "the same seam, key and all");
    assert_eq!(studio.state.selection, Selection::Seam(key));
}

#[test]
fn unpicking_a_seam_is_one_entry_and_undo_sews_it_back() {
    let mut studio = sewn();
    let (key, held) = only(&studio);
    studio.click(studio.centre(unpick_id(key)));
    studio.frame(Vec::new());
    assert_eq!(studio.doc().seams.len(), 0);
    assert_eq!(studio.session.undo_label(), Some("descoser"));
    assert_eq!(studio.state.selection, Selection::None);
    assert!(!says(&studio, "COSTURA 1"), "nothing is left to inspect");

    studio.session.undo().expect("the unpick steps back");
    assert_eq!(only(&studio), (key, held));
}

#[test]
fn the_sewing_tile_takes_the_tool_and_escape_puts_it_down() {
    let mut studio = Studio::new(unsewn());
    studio.frame(Vec::new());
    let tile = written(&studio, "Coser") + vec2(2.0, 2.0);
    studio.click(tile);
    assert_eq!(studio.state.tool, Tool::Sew);
    let (front, _) = front_and_back(studio.doc());
    let first = on_tract(&studio, front, "rodilla_lat");
    studio.click(first);
    assert!(matches!(studio.state.gesture, Gesture::Sewing(_)));
    studio.key(Key::Escape, Modifiers::NONE);
    assert_eq!(studio.state.gesture, Gesture::Idle, "the side is let go of");
    assert_eq!(studio.state.tool, Tool::Sew, "the tool is still in hand");
    studio.key(Key::Escape, Modifiers::NONE);
    assert_eq!(studio.state.tool, Tool::Select);
    assert_eq!(studio.session.revision(), 0, "none of it edited anything");
}

/// Sewing joins two pieces, so asking for the tool from one piece goes back to
/// the whole product; opening a piece puts the tool down again.
#[test]
fn the_sewing_key_leaves_a_piece_for_the_whole_product() {
    let mut studio = Studio::new(unsewn());
    let (front, _) = front_and_back(studio.doc());
    studio.state.open(front);
    studio.frame(Vec::new());
    studio.key(Key::S, Modifiers::NONE);
    assert_eq!(studio.state.scope, Scope::Product);
    assert_eq!(studio.state.tool, Tool::Sew);
    studio.state.open(front);
    assert_eq!(studio.state.tool, Tool::Select, "a piece has no use for it");
}
