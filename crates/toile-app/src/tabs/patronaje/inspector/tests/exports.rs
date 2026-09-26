use toile_engine::draft::block;

use super::super::exports::{paper_id, pdf_id, svg_id};
use super::desk::piece_open;
use crate::file::Action;

/// The size of paper is the one thing a print needs that no document holds, so
/// the panel that prints is where it is read and stepped. Whoever prints on
/// Carta has to be able to say so without leaving for a terminal.
#[test]
fn the_box_that_names_the_paper_asks_for_the_next_size() {
    let (mut desk, _) = piece_open(block::trousers());
    assert!(desk.drew(paper_id()), "the export section names the paper");
    let at = desk.centre(paper_id());
    desk.click(at);
    assert_eq!(desk.state.asked, Some(Action::Paper));
}

/// Every control of the section fits the panel: an exact-width panel does not
/// clip a row that overruns it, it draws it off the side of the window where
/// nothing can be pressed.
#[test]
fn the_section_fits_the_panel_it_is_drawn_in() {
    let (desk, _) = piece_open(block::trousers());
    let edge = desk
        .ctx
        .input(|i| i.raw.screen_rect)
        .expect("the desk was given a screen")
        .right();
    for id in [paper_id(), pdf_id(), svg_id()] {
        assert!(desk.rect(id).right() <= edge, "{:?}", desk.rect(id));
    }
}

/// Both ways out answer a press, and both are over the whole product: a button
/// in this row that is drawn but not wired promises a file nobody writes, and
/// that is what this asserts against.
#[test]
fn both_ways_out_of_the_studio_answer_a_press() {
    let (mut desk, _) = piece_open(block::trousers());
    for (id, asked) in [(pdf_id(), Action::Pdf), (svg_id(), Action::Svg)] {
        desk.state.asked = None;
        let at = desk.centre(id);
        desk.click(at);
        assert_eq!(desk.state.asked, Some(asked));
    }
}
