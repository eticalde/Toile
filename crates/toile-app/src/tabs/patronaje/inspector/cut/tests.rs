use super::*;

/// The box shows the width the file holds, with nothing to type past.
#[test]
fn a_width_reads_as_the_centimetres_somebody_would_write() {
    assert_eq!(centimetres(1.5), "1.5");
    assert_eq!(centimetres(1.0), "1");
    assert_eq!(centimetres(0.0), "0", "cut on the line, which is a width");
    assert_eq!(centimetres(10.0), "10", "and the tens survive the trim");
    assert_eq!(centimetres(0.75), "0.75");
}

/// An empty box is no width at all, a number outside the document's gate
/// is nothing to ask for, and zero is a width like any other.
#[test]
fn an_empty_box_asks_for_a_net_piece_and_a_bad_number_asks_for_nothing() {
    assert_eq!(width(""), Some(Width::Net));
    assert_eq!(width("0"), Some(Width::Outside(0.0)));
    assert_eq!(width("1.5"), Some(Width::Outside(1.5)));
    assert_eq!(width("-1.5"), None, "a margin is cloth outside the line");
    assert_eq!(width("inf"), None);
    assert_eq!(width("uno"), None);
}

/// A line written over, a line added past the end, a line emptied away.
#[test]
fn a_label_takes_a_line_written_over_it_or_past_it_or_out_of_it() {
    let held = ["DELANTERO".to_owned(), "cortar 2".to_owned()];
    let over = lines(&held, 1, "cortar 2 espejadas".to_owned());
    assert_eq!(over, ["DELANTERO", "cortar 2 espejadas"]);
    let past = lines(&held, 2, "ojales".to_owned());
    assert_eq!(past, ["DELANTERO", "cortar 2", "ojales"]);
    assert_eq!(lines(&held, 0, String::new()), ["cortar 2"]);
}
