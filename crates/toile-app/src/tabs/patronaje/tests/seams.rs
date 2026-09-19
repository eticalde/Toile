use eframe::egui::{Event, Key, Modifiers};
use toile_engine::draft::{Command, EdgeRange, Identity, Seam, SeamOrientation, block};

use super::super::inspector::seams::row_id;
use super::super::state::Selection;
use super::bench::front_and_back;
use super::sewing::{on_tract, only, says, unsewn};
use super::studio::Studio;

/// Looking at seams is not editing them: reading the list, choosing a seam,
/// hovering its sides, picking a first side and letting it go all leave the
/// file the file that was opened.
#[test]
fn looking_at_the_seams_of_a_product_writes_nothing() {
    let shipped = block::trousers();
    let mut studio = Studio::new(shipped.clone());
    studio.frame(Vec::new());
    assert!(
        says(&studio, "1 · Delantero — Trasero"),
        "the seams are listed"
    );
    let first = studio.doc().seams.keys().next().expect("the block is sewn");
    studio.click(studio.centre(row_id(first)));
    studio.frame(Vec::new());
    assert_eq!(studio.state.selection, Selection::Seam(first));
    assert!(says(&studio, "A · Delantero · cintura_lat → bajo_lat"));
    assert!(says(&studio, "dentro de la tolerancia"), "and measured");
    studio.click(studio.centre(row_id(first)));
    assert_eq!(
        studio.state.selection,
        Selection::None,
        "a second press lets go"
    );

    let (front, back) = front_and_back(studio.doc());
    studio.key(Key::S, Modifiers::NONE);
    for (piece, from) in [(front, "cadera_lat"), (back, "cadera_lat_tras")] {
        let at = on_tract(&studio, piece, from);
        studio.frame(vec![Event::PointerMoved(at)]);
    }
    studio.click(on_tract(&studio, front, "rodilla_lat"));
    studio.key(Key::Escape, Modifiers::NONE);

    assert_eq!(studio.session.revision(), 0, "no edit was played");
    assert!(
        !studio.session.can_undo(),
        "and nothing reached the history"
    );
    assert_eq!(
        studio.doc().to_canonical_json(),
        shipped.to_canonical_json()
    );
}

/// A seam the engine cannot pair is said next to that seam, in the list and
/// under it, instead of draping a garment that is quietly one seam short.
#[test]
fn a_seam_the_engine_cannot_pair_says_so_in_the_inspector() {
    let mut doc = unsewn();
    let (front, back) = front_and_back(&doc);
    let node = |piece, label| doc.shows_label(piece, label).expect("a named node");
    let a = EdgeRange::between(front, node(front, "rodilla_lat"), node(front, "bajo_lat"));
    let pinched = node(back, "rodilla_lat_tras");
    let b = EdgeRange::between(back, pinched, pinched);
    Command::AddSeam {
        identity: Identity::New,
        seam: Seam::plain(a, b, SeamOrientation::Aligned),
    }
    .apply(&mut doc)
    .expect("the document takes a side with no length");

    let mut studio = Studio::new(doc);
    studio.frame(Vec::new());
    assert_eq!(
        studio.session.seam_faults().len(),
        1,
        "the engine refuses it"
    );
    assert!(says(&studio, "no llega a la tela"), "said on its row");
    let (key, _) = only(&studio);
    studio.click(studio.centre(row_id(key)));
    studio.frame(Vec::new());
    assert!(says(
        &studio,
        "No llega a la tela: los dos extremos caen en el mismo punto."
    ));
}
