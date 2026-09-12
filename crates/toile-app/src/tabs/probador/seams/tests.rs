use toile_engine::draft::{Binding, Draft, block};

use super::*;

/// The table is the document's own seams, measured along the contours they
/// join — not a count and not a length anybody wrote down.
#[test]
fn the_table_counts_and_measures_the_seams_the_document_holds() {
    let doc = block::trousers();
    let held = doc.seams.len();
    let draft = Draft::from_doc(doc).expect("the block resolves");
    let rows = measured(&draft);

    assert_eq!(rows.len(), held);
    assert_eq!(rows[0].name, "cintura_lat → bajo_lat");
    // The front's side seam is 104.6 cm along the flattening, and the back is
    // cut to close on it to within the block's own tolerance.
    assert!(
        rows[0].lengths.starts_with("104.6 / 104."),
        "{}",
        rows[0].lengths
    );
    for row in &rows {
        assert_eq!(row.meets, Some(true), "{}", row.name);
        assert_eq!(row.complaint, None, "{}", row.name);
    }
}

/// A seam whose two sides stop agreeing is the only one that complains, and it
/// complains about itself.
#[test]
fn a_side_that_stops_closing_is_the_only_one_that_says_so() {
    let mut doc = block::trousers();
    let back = doc
        .piece_named(block::BACK)
        .expect("the block draws a back");
    let hem = doc
        .shows_label(back, "bajo_lat_tras")
        .expect("the back names its hem at the side");
    let settled = Draft::from_doc(doc.clone()).expect("the block resolves");
    let y = settled.resolved(hem).expect("the hem resolves")[1];
    doc.points.get_mut(hem).expect("the key is live").y = Binding::Literal(y + 3.0);
    let draft = Draft::from_doc(doc).expect("a longer leg still resolves");

    let rows = measured(&draft);
    assert_eq!(rows[0].meets, Some(false));
    let complaint = rows[0]
        .complaint
        .as_deref()
        .expect("a seam that does not close says so");
    assert!(
        complaint.starts_with("cintura_lat → bajo_lat: los largos difieren"),
        "{complaint}"
    );
    assert_eq!(rows[1].meets, Some(true), "the inseam was not touched");
    assert_eq!(rows[1].complaint, None);
}
