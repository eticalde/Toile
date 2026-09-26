use toile_doc::{
    Command, Doc, EdgeRange, FORMAT_VERSION_FOLDED, FormatError, History, Identity, Symmetry, block,
};

use super::elastic::elasticated;
use super::line::lined;
use super::placement::placed;
use super::{EMPTY, SHIPPED, header, linked, restamped, rewritten, shaped};

/// The edit that folds the shipped block's front on its waistline.
///
/// Geometric nonsense as a garment — the waist is not a fold — but the format
/// does not measure, and every case here is about the stamp.
fn folded_front(doc: &Doc) -> Command {
    let front = doc.piece_named(block::FRONT).expect("the block draws one");
    let named = |label| doc.shows_label(front, label).expect("the block names it");
    let axis = EdgeRange::between(front, named("cintura_cf"), named("cintura_lat"));
    Command::AddSymmetry {
        identity: Identity::New,
        symmetry: Symmetry::fold(axis),
    }
}

/// The shipped block with that fold on it, stamped `version`.
pub(super) fn folded(version: u32) -> String {
    let mut doc = block::trousers();
    folded_front(&doc)
        .apply(&mut doc)
        .expect("both ends are nodes of the front");
    restamped(&doc.to_canonical_json(), version)
}

/// Every file on disk was written before a piece could be drawn against an
/// axis, and reading one in a build that knows the fold may not change a byte
/// of it, the stamp least of all. The empty arena was always written, so there
/// is no new field to leave out: what has to hold still is the stamp.
#[test]
fn every_file_written_before_the_fold_re_saves_byte_identical() {
    for file in [
        SHIPPED.to_owned(),
        EMPTY.to_owned(),
        shaped(2),
        linked(3, false),
        placed(SHIPPED, 4),
        elasticated(5),
        lined(6),
    ] {
        assert!(
            file.contains("\"symmetries\": {\n      \"issued\": 0,"),
            "{file}"
        );
        assert_eq!(rewritten(&file).as_bytes(), file.as_bytes());
    }
}

/// A fold takes the number after the internal line's. Builds that read version
/// 6 predate it: stamped 6, a folded product would open there as the drawn half
/// alone — cut, meshed, draped and exported at half its width — and saving
/// would make that for good.
#[test]
fn the_version_stamp_is_7_exactly_when_a_piece_is_drawn_against_an_axis() {
    for (file, stamp) in [
        (folded(1), 7),
        (folded(6), 7),
        (folded(7), 7),
        (restamped(SHIPPED, 7), 1),
        (restamped(&shaped(2), 7), 2),
        (restamped(&placed(SHIPPED, 4), 7), 4),
        (restamped(&lined(6), 7), 6),
    ] {
        let doc = Doc::from_json(&file).unwrap_or_else(|error| panic!("{error}: {file}"));
        assert_eq!(doc.format_version(), stamp, "{file}");
        assert!(doc.to_canonical_json().starts_with(&header(stamp)));
    }
}

#[test]
fn a_version_7_document_round_trips_byte_identical() {
    let file = folded(7);
    assert!(file.contains("\"kind\": \"fold\""), "{file}");
    assert!(file.contains("\"head\": {"), "{file}");
    assert_eq!(rewritten(&file).as_bytes(), file.as_bytes());
    let doc = Doc::from_json(&file).expect("this build reads version 7");
    assert_eq!(Doc::from_json(&doc.to_canonical_json()), Ok(doc));
}

/// The stamp follows the content through an undo as well: a piece unfolded
/// again is written at the version it was written at before.
///
/// Not byte for byte, and that is the arena's doing and not the fold's: the
/// store of axes has always been written, so the slot the fold took stays
/// counted, exactly as a seam's or a notch's does. What the fold owes is the
/// stamp, and the stamp comes back.
#[test]
fn a_fold_made_and_undone_takes_the_file_to_version_7_and_back() {
    let mut doc = block::trousers();
    let before = doc.to_canonical_json();
    let mut history = History::new();
    let edit = folded_front(&doc);
    history
        .edit(&mut doc, edit)
        .expect("both ends are nodes of the front");
    assert_eq!(doc.format_version(), FORMAT_VERSION_FOLDED);
    let file = doc.to_canonical_json();
    assert!(file.starts_with(&header(7)));

    history.undo(&mut doc).expect("the axis is live");
    let undone = doc.to_canonical_json();
    assert!(undone.starts_with(&header(1)), "the stamp comes back");
    assert_eq!(
        undone.replace(
            "\"issued\": 1,\n      \"entries\": []",
            "\"issued\": 0,\n      \"entries\": []"
        ),
        before,
        "and so does everything but the slot the axis was counted in"
    );
    history.redo(&mut doc).expect("the slot is free");
    assert_eq!(doc.to_canonical_json(), file);
}

/// `folded(7)` with `from` replaced by `to` inside the axis store only.
///
/// The anchors of a seam are written with the same words at the same depth, and
/// they come first in the file, so a plain replacement would edit the seam and
/// leave the axis alone.
fn hand_edited(from: &str, to: &str) -> String {
    let file = folded(7);
    let at = file.find("\"symmetries\"").expect("the store is written");
    let (head, store) = file.split_at(at);
    assert!(store.contains(from), "the fixture moved under the test");
    format!("{head}{}", store.replacen(from, to, 1))
}

/// The file is the other way in, so an axis no piece can be repeated across is
/// refused there too: a fraction off the end of its tract, and a mirror, which
/// nothing cuts the second piece of yet.
#[test]
fn an_axis_no_piece_can_be_repeated_across_is_refused_before_the_pattern_opens() {
    for (from, to, says) in [
        (
            "\"kind\": \"fold\"",
            "\"kind\": \"mirror\"",
            "not implemented",
        ),
        ("\"t\": 0", "\"t\": 3", "0 to 1"),
    ] {
        let broken = hand_edited(from, to);
        let Err(error) = Doc::from_json(&broken) else {
            panic!("no piece is repeated across {to}");
        };
        assert!(matches!(error, FormatError::Symmetry(_)), "{error}");
        assert!(error.to_string().contains(says), "{error}");
    }
}
