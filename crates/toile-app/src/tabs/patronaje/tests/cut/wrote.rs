use eframe::egui::{Key, Modifiers};
use toile_engine::draft::PieceKey;

use super::super::studio::Studio;
use super::{Cut, box_of, delantero, entries, held, says, tira};

/// Types `text` over whatever one of the piece's cut boxes holds, and
/// confirms it.
///
/// Select-all then type, because every box comes up holding what the document
/// holds: typing into it without that would append to the value instead of
/// replacing it, which is not what a hand on a keyboard does.
fn write(studio: &mut Studio, piece: PieceKey, of: Cut, text: &str) {
    studio.click(studio.centre(box_of(piece, of)));
    studio.key(Key::A, Modifiers::COMMAND);
    if text.is_empty() {
        studio.key(Key::Backspace, Modifiers::NONE);
    } else {
        studio.text(text);
    }
    studio.key(Key::Enter, Modifiers::NONE);
    studio.frame(Vec::new());
}

/// A margin typed onto a net piece, in one entry of its own, and one undo
/// takes the piece back to being net.
#[test]
fn a_margin_typed_is_one_entry_and_undo_leaves_the_piece_net_again() {
    let (mut studio, piece) = tira();
    assert_eq!(held(&studio, piece).allowance, None, "the strip starts net");

    write(&mut studio, piece, Cut::Allowance, "1.5");
    assert_eq!(held(&studio, piece).allowance, Some(1.5));
    assert_eq!(entries(&studio), 1, "one box confirmed, one entry");
    assert_eq!(studio.session.undo_label(), Some("escribir el margen"));
    assert!(says(&studio, "cm por fuera del contorno"));

    studio.session.undo().expect("the entry steps back");
    studio.frame(Vec::new());
    assert_eq!(held(&studio, piece).allowance, None);
    assert!(says(&studio, "neta: se corta por el contorno"));
}

/// Emptying the box makes the piece net, and not a piece cut on its line.
///
/// `None` and `0.0` are two different sentences — nobody wrote a width, and
/// the cloth is cut on the very line it is sewn on — so the empty box has to
/// land on the first of them.
#[test]
fn emptying_the_margin_box_is_no_width_at_all_and_not_a_width_of_zero() {
    let (mut studio, piece) = delantero();
    write(&mut studio, piece, Cut::Allowance, "");
    assert_eq!(held(&studio, piece).allowance, None);
    let bytes = studio.doc().to_canonical_json();
    assert!(
        !bytes.contains("seamAllowance"),
        "a net piece writes no width: {bytes}"
    );
}

/// A count of none is refused under the box, and the document never sees it.
///
/// Said in Spanish on the panel instead of played and refused: the edit would
/// come back as a line in the status bar after a round trip, and the box would
/// go on showing a number the document does not hold.
#[test]
fn a_count_of_none_is_refused_in_the_box_it_was_typed_in() {
    let (mut studio, piece) = delantero();
    let bytes = studio.doc().to_canonical_json();
    write(&mut studio, piece, Cut::Quantity, "0");

    assert!(says(&studio, "un entero desde 1"));
    assert_eq!(held(&studio, piece).quantity, 2, "the count did not move");
    assert_eq!(studio.doc().to_canonical_json(), bytes);
    assert_eq!(entries(&studio), 0, "nothing reached the history");
}

/// The letter is free text, and an empty box is no letter at all.
#[test]
fn the_letter_takes_what_somebody_wrote_on_the_paper_and_gives_it_back() {
    let (mut studio, piece) = delantero();
    write(&mut studio, piece, Cut::Letter, "J");
    assert_eq!(held(&studio, piece).letter.as_deref(), Some("J"));
    assert_eq!(studio.session.undo_label(), Some("escribir la letra"));

    write(&mut studio, piece, Cut::Letter, "");
    assert_eq!(
        held(&studio, piece).letter,
        None,
        "no letter, not an empty one"
    );
    assert_eq!(entries(&studio), 2);
}

/// A line typed in the last row goes on the end of the label, and a line
/// emptied comes off it.
#[test]
fn the_last_row_adds_a_line_of_the_label_and_an_emptied_row_takes_one_off() {
    let (mut studio, piece) = delantero();
    write(&mut studio, piece, Cut::Label(2), "ojales");
    assert_eq!(
        held(&studio, piece).labels,
        ["DELANTERO", "cortar 2 espejadas", "ojales"]
    );
    assert_eq!(studio.session.undo_label(), Some("escribir el rótulo"));
    assert!(says(&studio, "rótulo 3"));

    write(&mut studio, piece, Cut::Label(1), "");
    assert_eq!(held(&studio, piece).labels, ["DELANTERO", "ojales"]);
    assert!(!says(&studio, "cortar 2 espejadas"));
}

/// The line and the count are two things one person wrote, and the panel
/// holds both without reading either into the other.
///
/// SOLAPA TRASERA is the owner's own defect: the label asks for two and the
/// garment takes four. A panel that read the line would have written 4 here,
/// which is a number nobody put in the file.
#[test]
fn a_line_that_names_a_number_never_tells_the_count_what_to_be() {
    let (mut studio, piece) = delantero();
    write(&mut studio, piece, Cut::Label(1), "cortar 2 + entretela");
    assert_eq!(
        held(&studio, piece).quantity,
        2,
        "the count stayed where it was"
    );
    assert!(says(&studio, "cortar 2 + entretela"));

    write(&mut studio, piece, Cut::Label(1), "cortar 4 + entretela");
    assert_eq!(
        held(&studio, piece).quantity,
        2,
        "and still nobody read the line"
    );
    assert_eq!(held(&studio, piece).labels[1], "cortar 4 + entretela");
}

/// A click in and out of a box leaves no entry behind.
///
/// Every box comes up holding what the document holds, and losing the focus is
/// how a box confirms; writing that back would leave a step in the history
/// that undoes to itself.
#[test]
fn a_click_in_and_out_of_a_box_that_changed_nothing_writes_nothing() {
    let (mut studio, piece) = delantero();
    let revision = studio.session.revision();
    studio.click(studio.centre(box_of(piece, Cut::Quantity)));
    studio.click(studio.centre(box_of(piece, Cut::Letter)));
    studio.click(studio.centre(box_of(piece, Cut::Label(0))));
    studio.frame(Vec::new());

    assert_eq!(studio.session.revision(), revision, "no edit was played");
    assert_eq!(entries(&studio), 0);
}

/// A box half written on one piece is let go of when another comes in front.
///
/// Kept, its text would come back in that box the next time the piece is in
/// front, and the first click in and out of it would write something nobody
/// confirmed. It is the rule the coordinate rows already keep.
#[test]
fn a_box_half_written_is_let_go_of_when_another_piece_comes_in_front() {
    let (mut studio, piece) = delantero();
    studio.click(studio.centre(box_of(piece, Cut::Quantity)));
    studio.key(Key::A, Modifiers::COMMAND);
    studio.text("7");
    assert!(studio.state.editing.is_some(), "the box is being written");

    let other = studio
        .doc()
        .piece_keys()
        .into_iter()
        .find(|&key| key != piece)
        .expect("the trousers draw two pieces");
    studio.state.front(other);
    studio.frame(Vec::new());

    assert_eq!(studio.state.editing, None, "the text went with the piece");
    assert_eq!(
        held(&studio, piece).quantity,
        2,
        "and never reached the document"
    );
}
