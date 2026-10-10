use eframe::egui::{Key, Modifiers, Rect};
use toile_engine::draft::block;

use super::super::super::state::{Cut, Selection};
use super::super::bench::front_and_back;
use super::super::studio::Studio;
use super::{box_of, delantero, drew, inside, says, tira, words};

/// The whole of what the owner's piece says about being cut out, on the glass.
///
/// Every value read back out of the box the frame painted it in, and every row
/// read back by the name beside it: the letter, the count, the width with the
/// line that says where it is measured from, and both lines of the label in
/// the order their writer put them in.
#[test]
fn the_panel_shows_what_the_owners_piece_says_about_being_cut_out() {
    let (studio, front) = delantero();
    let in_box = |of| inside(&studio, box_of(front, of));
    assert_eq!(in_box(Cut::Letter).as_deref(), Some("A"));
    assert_eq!(in_box(Cut::Quantity).as_deref(), Some("2"));
    assert_eq!(in_box(Cut::Allowance).as_deref(), Some("1.5"));
    assert_eq!(in_box(Cut::Label(0)).as_deref(), Some("DELANTERO"));
    assert_eq!(in_box(Cut::Label(1)).as_deref(), Some("cortar 2 espejadas"));
    let add = in_box(Cut::Label(2));
    assert_eq!(add.as_deref(), Some(""), "the row that adds holds nothing");
    for want in [
        "CORTE",
        "letra",
        "cortar",
        "margen",
        "cm por fuera del contorno",
        "rótulo 1",
        "rótulo 2",
        "añadir",
    ] {
        assert!(says(&studio, want), "«{want}» is not on the panel");
    }
}

/// A net piece shows an empty box that says what empty means.
///
/// Two of the owner's ten are cut on their own line, and «none» there is a
/// different claim from a width of zero: a box with nothing in it and no line
/// under it would read as a number somebody forgot.
#[test]
fn a_net_piece_says_it_is_cut_on_the_contour_it_is_drawn_to() {
    let (studio, piece) = tira();
    let shown = inside(&studio, box_of(piece, Cut::Allowance));
    assert_eq!(shown.as_deref(), Some(""), "no width, so no number to read");
    assert!(says(&studio, "neta: se corta por el contorno"));
    assert!(
        !says(&studio, "cm por fuera del contorno"),
        "there is no width to measure from anywhere"
    );
    assert!(says(&studio, "cortar 1 - doblar en tercios"));
    let letter = inside(&studio, box_of(piece, Cut::Letter));
    assert_eq!(letter.as_deref(), Some("F"), "the letter its writer gave");
}

/// A piece nobody wrote a cut for still shows every box.
///
/// The section is not a reader of imported patterns: a piece drawn on the
/// table opens with a count of one, no letter, no allowance and no label, and
/// each of those is a box to write in rather than a row that is missing.
#[test]
fn a_piece_that_says_nothing_about_being_cut_shows_the_boxes_to_say_it_in() {
    let shipped = block::trousers();
    let bytes = shipped.to_canonical_json();
    let mut studio = Studio::new(shipped);
    studio.frame(Vec::new());
    let (front, _) = front_and_back(studio.doc());

    assert!(says(&studio, "CORTE"));
    let count = inside(&studio, box_of(front, Cut::Quantity));
    assert_eq!(
        count.as_deref(),
        Some("1"),
        "cut once is what silence means"
    );
    assert_eq!(
        inside(&studio, box_of(front, Cut::Letter)).as_deref(),
        Some("")
    );
    assert!(says(&studio, "neta: se corta por el contorno"));
    assert!(says(&studio, "añadir"), "the row a first line goes in");
    assert!(
        !says(&studio, "rótulo 1"),
        "no line, so no row claiming one"
    );
    for of in [Cut::Letter, Cut::Quantity, Cut::Allowance, Cut::Label(0)] {
        assert!(drew(&studio, box_of(front, of)), "{of:?}");
    }

    studio.frame(Vec::new());
    assert_eq!(studio.session.revision(), 0, "looking wrote nothing");
    assert!(!studio.session.can_undo());
    assert_eq!(studio.doc().to_canonical_json(), bytes);
}

/// The boxes are the piece's own, so they stay through every selection, and
/// so does a box half written.
///
/// They are not an answer to a press: the piece in front is what they belong
/// to, and a section that came and went as nodes were chosen would move the
/// rows under the hand that was reaching for them. A row that goes on being
/// drawn goes on holding its text, which is the contract every row of this
/// panel keeps.
#[test]
fn the_boxes_belong_to_the_piece_and_stay_through_whatever_is_chosen() {
    let (mut studio, front) = delantero();
    let node = studio
        .doc()
        .shows_label(front, "cadera_lat")
        .expect("the block names the hip");
    studio.click(studio.centre(box_of(front, Cut::Quantity)));
    studio.key(Key::A, Modifiers::COMMAND);
    studio.text("7");
    for chosen in [
        Selection::point(node),
        Selection::Edge(node),
        Selection::None,
    ] {
        studio.state.choose(chosen.clone());
        studio.frame(Vec::new());
        for of in [Cut::Letter, Cut::Quantity, Cut::Allowance, Cut::Label(0)] {
            assert!(drew(&studio, box_of(front, of)), "{chosen:?} {of:?}");
        }
        let held = studio
            .state
            .editing
            .as_ref()
            .map(|edit| edit.buffer.as_str());
        assert_eq!(
            held,
            Some("7"),
            "{chosen:?}: the box is still being written"
        );
    }
    assert_eq!(studio.session.revision(), 0, "and nothing was confirmed");
}

/// Choosing a seam still opens the inspector on it, with this section on the
/// panel.
///
/// The reason the section is drawn under the ones that answer a press: on a
/// window this tall the seam's own detail already reaches the foot of the
/// glass, so a block of boxes above it would have pushed the answer out of
/// sight on the very press that asked the question.
#[test]
fn choosing_a_seam_still_shows_its_own_detail_on_the_glass() {
    let (mut studio, _) = delantero();
    let key = studio
        .doc()
        .seams
        .keys()
        .next()
        .expect("the block is sewn up");
    studio.state.choose(Selection::Seam(key));
    studio.frame(Vec::new());

    assert!(says(&studio, "COSTURA 1"), "the inspector opens on it");
    assert!(
        words(&studio)
            .iter()
            .any(|line| line.contains("dentro de la tolerancia")),
        "and its verdict is where a person can read it"
    );
}

/// The rows of the section stack instead of landing on one another.
///
/// The bug this guards is the tab's oldest: a row that places a widget inside
/// the room it took and does not take that room back hands it to the row
/// below, which is then drawn over the top of it.
#[test]
fn every_box_of_the_section_sits_under_the_one_before_it() {
    let (studio, front) = delantero();
    let mut last: Option<Rect> = None;
    for of in [
        Cut::Letter,
        Cut::Quantity,
        Cut::Allowance,
        Cut::Label(0),
        Cut::Label(1),
        Cut::Label(2),
    ] {
        let rect = studio
            .ctx
            .read_response(box_of(front, of))
            .expect("the panel drew it")
            .rect;
        if let Some(above) = last {
            assert!(above.bottom() < rect.top(), "{of:?}: {above:?} {rect:?}");
        }
        assert!(
            rect.width() > 100.0,
            "{of:?}: a box wide enough to write in"
        );
        last = Some(rect);
    }
}
