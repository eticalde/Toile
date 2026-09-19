use eframe::egui::{Key, Modifiers};
use toile_engine::draft::{Doc, EdgeRange, PieceKey, Seam, SeamOrientation, block};

use super::super::gesture::Gesture;
use super::super::sew::{APART, Pick, Sewing};
use super::super::state::Selection;
use super::super::status;
use super::bench::front_and_back;
use super::sewing::{on_tract, only, says, unsewn};
use super::studio::Studio;

/// The three tracts of the block's side seam, by the node each leaves: waist
/// to hip, hip to knee, knee to hem. The back names the same nodes with
/// `_tras` after them.
const SIDE: [&str; 3] = ["cintura_lat", "cadera_lat", "rodilla_lat"];

/// The whole side of one piece, waist to hem, as one stretch.
fn side(doc: &Doc, piece: PieceKey, suffix: &str) -> EdgeRange {
    let node = |label: &str| {
        let label = format!("{label}{suffix}");
        doc.shows_label(piece, &label).expect("a named node")
    };
    EdgeRange::between(piece, node("cintura_lat"), node("bajo_lat"))
}

/// A real side seam is one seam: three tracts of the front, picked as one side
/// with shift, sewn to the back's three, and the engine pairs the whole of it.
#[test]
fn a_side_of_three_tracts_is_sewn_as_one_seam_and_reaches_the_cloth() {
    let mut studio = Studio::new(unsewn());
    studio.frame(Vec::new());
    studio.key(Key::S, Modifiers::NONE);
    let (front, back) = front_and_back(studio.doc());
    studio.click(on_tract(&studio, front, SIDE[0]));
    for from in &SIDE[1..] {
        studio.shift_click(on_tract(&studio, front, from));
    }
    let first = Pick::of(side(studio.doc(), front, "")).expect("node to node");
    assert_eq!(
        studio.state.gesture,
        Gesture::Sewing(Sewing { first, pan: None }),
        "three tracts in hand as one stretch"
    );
    assert_eq!(studio.session.revision(), 0, "and nothing written for them");

    studio.click(on_tract(&studio, back, "cintura_lat_tras"));
    studio.frame(Vec::new());
    let (key, held) = only(&studio);
    assert_eq!(
        held.a,
        side(studio.doc(), front, ""),
        "one range, not three"
    );
    assert_eq!(studio.state.selection, Selection::Seam(key));
    for from in &SIDE[1..] {
        studio.shift_click(on_tract(&studio, back, &format!("{from}_tras")));
        studio.frame(Vec::new());
    }

    let doc = studio.doc();
    let whole = Seam::plain(
        side(doc, front, ""),
        side(doc, back, "_tras"),
        SeamOrientation::Aligned,
    );
    assert_eq!(only(&studio), (key, whole), "the block's own side seam");
    let shipped = block::trousers();
    let theirs = shipped.seams.iter().next().expect("the block is sewn").1;
    assert_eq!(
        (held.orientation, whole.kind),
        (theirs.orientation, theirs.kind)
    );
    assert_eq!(
        studio.session.undo_label(),
        Some("ajustar un lado de la costura")
    );
    assert!(says(&studio, "dentro de la tolerancia"), "and it closes");

    studio.session.wait_for_remesh().expect("both pieces mesh");
    assert_eq!(studio.session.seam_faults(), &[], "the engine pairs it");
    assert!(!studio.session.sewn_pairs().is_empty());

    studio
        .session
        .undo()
        .expect("the last tract steps back off");
    assert_ne!(only(&studio).1.b, whole.b, "one tract shorter on side B");
    assert_eq!(only(&studio).1.a, whole.a);
}

/// A side is one stretch of contour, so a tract that does not touch it is not
/// added and not dropped in silence either: the bar says why, the side stays
/// in hand, and the next press that lands takes the refusal away.
#[test]
fn a_tract_apart_from_the_side_in_hand_is_refused_in_the_status_bar() {
    let mut studio = Studio::new(unsewn());
    studio.frame(Vec::new());
    studio.key(Key::S, Modifiers::NONE);
    let (front, back) = front_and_back(studio.doc());
    studio.click(on_tract(&studio, front, SIDE[0]));
    let held = studio.state.gesture.clone();

    for (piece, from) in [(front, SIDE[2]), (back, "cintura_lat_tras")] {
        studio.shift_click(on_tract(&studio, piece, from));
        assert_eq!(studio.state.gesture, held, "the side picked is kept");
        assert_eq!(studio.state.refused.as_deref(), Some(APART));
        let bar = status(&studio.session, &studio.state);
        let said = format!("rechazado: {APART}");
        assert!(bar.contains(&(said, true)), "said as an alert: {bar:?}");
    }
    assert_eq!(studio.session.revision(), 0, "a refusal writes nothing");

    studio.shift_click(on_tract(&studio, front, SIDE[1]));
    assert_eq!(studio.state.refused, None, "a press that lands clears it");
    assert_ne!(studio.state.gesture, held, "and the side is a tract longer");
}
