use toile_doc::{
    Command, Doc, DocError, FORMAT_VERSION_HEADED, FORMAT_VERSION_HUNG, Hang, HangKey, Heading,
    History, Sense, block,
};

use super::cut::cut;
use super::dart::darted;
use super::elastic::elasticated;
use super::hang::{hang, hung};
use super::line::lined;
use super::placement::placed;
use super::symmetry::folded;
use super::{EMPTY, SHIPPED, header, linked, restamped, rewritten, shaped};

/// The heading every scene here declares: the waistline's head at the centre
/// front, running toward the wearer's left.
fn heading() -> Heading {
    Heading::facing(0.0, Sense::Leftward)
}

/// The shipped block hung from the waist and turned to face the front, stamped
/// `version`.
pub(super) fn headed(version: u32) -> String {
    let mut doc = block::trousers();
    hang(&doc)
        .apply(&mut doc)
        .expect("both ends are nodes of the front");
    let key = only(&doc);
    Command::SetHangHeading {
        hang: key,
        to: Some(heading()),
    }
    .apply(&mut doc)
    .expect("the hang is live");
    restamped(&doc.to_canonical_json(), version)
}

/// The one hang a scene here writes.
fn only(doc: &Doc) -> HangKey {
    let (key, _) = doc
        .hangs
        .iter()
        .next()
        .expect("the scene hangs the waistline");
    key
}

/// Every file on disk was written before a hang could say which way it faces,
/// and reading one in a build that knows the heading may not change a byte of
/// it, the stamp least of all.
///
/// The hung file is in the list and is the one that matters: the heading is a
/// field of a hang, so this is the whole of the conditional rule — a document
/// that hangs and declares nothing is still a version 8 document, byte for
/// byte.
#[test]
fn every_file_written_before_the_heading_re_saves_byte_identical() {
    for file in [
        SHIPPED.to_owned(),
        EMPTY.to_owned(),
        shaped(2),
        linked(3, false),
        placed(SHIPPED, 4),
        elasticated(5),
        lined(6),
        folded(7),
        hung(8),
        darted(9),
        cut(SHIPPED, 10),
    ] {
        assert!(!file.contains("heading"), "{file}");
        assert_eq!(rewritten(&file).as_bytes(), file.as_bytes());
    }
}

/// The heading takes the number after the cut's, and takes it only when a hang
/// declares one.
///
/// Both halves in one table. A hung document with no heading stays at 8, which
/// is what keeps every pattern already drawn where it is; one with a heading
/// asks for 11, because a build that reads 10 would drop it and then open the
/// product with the turn starting at whichever panel the file holds first.
#[test]
fn the_version_stamp_is_11_exactly_when_a_hang_says_which_way_it_faces() {
    for (file, stamp) in [
        (headed(1), FORMAT_VERSION_HEADED),
        (headed(10), FORMAT_VERSION_HEADED),
        (headed(11), FORMAT_VERSION_HEADED),
        (restamped(&hung(8), 11), FORMAT_VERSION_HUNG),
        (restamped(SHIPPED, 11), 1),
        (restamped(&cut(SHIPPED, 10), 11), 10),
    ] {
        let doc = Doc::from_json(&file).unwrap_or_else(|error| panic!("{error}: {file}"));
        assert_eq!(doc.format_version(), stamp, "{file}");
        assert!(doc.to_canonical_json().starts_with(&header(stamp)));
    }
}

#[test]
fn a_version_11_document_round_trips_byte_identical() {
    let file = headed(11);
    // The shortest spelling that reads back as itself, which is what the
    // canonical writer gives every number: a heading at the centre front with
    // its head pinned is three fields and two bare zeroes.
    assert!(file.contains("\"pin\": 0,\n"), "{file}");
    assert!(file.contains("\"turn\": 0,\n"), "{file}");
    assert!(file.contains("\"sense\": \"Leftward\""), "{file}");
    assert_eq!(rewritten(&file).as_bytes(), file.as_bytes());
    let doc = Doc::from_json(&file).expect("this build reads version 11");
    assert_eq!(Doc::from_json(&doc.to_canonical_json()), Ok(doc));
}

/// A turn hand-typed outside the one lap there is is refused at the door, for
/// the reason a hand-typed elastic ratio is: it reaches the placement as an
/// angle to roll the cloth by, and a turn of 400 names a place three quarters
/// round from where it reads.
#[test]
fn a_turn_the_file_carries_is_checked_before_the_pattern_opens() {
    let file = headed(11);
    for turn in ["400.0", "-180.0", "180.5"] {
        let edited = file.replacen("\"turn\": 0,", &format!("\"turn\": {turn},"), 1);
        assert_ne!(edited, file, "{turn} left the file as it was");
        let error = Doc::from_json(&edited).expect_err(turn);
        assert!(error.to_string().contains("180"), "{turn}: {error}");
    }
    let edited = file.replacen("\"pin\": 0,", "\"pin\": 2.0,", 1);
    assert_ne!(edited, file, "the file carries a pin");
    let error = Doc::from_json(&edited).expect_err("a pin of 2 names no point of the run");
    assert!(error.to_string().contains("0 to 1"), "{error}");
}

/// The stamp follows the content through an undo: a hang whose heading has been
/// taken back off is the version 8 file it was, byte for byte.
#[test]
fn a_heading_put_on_and_undone_takes_the_file_to_version_11_and_back() {
    let mut doc = block::trousers();
    hang(&doc)
        .apply(&mut doc)
        .expect("both ends are nodes of the front");
    let before = doc.to_canonical_json();
    assert!(before.starts_with(&header(FORMAT_VERSION_HUNG)));

    let mut history = History::new();
    let edit = Command::SetHangHeading {
        hang: only(&doc),
        to: Some(heading()),
    };
    history.edit(&mut doc, edit).expect("the hang is live");
    assert_eq!(doc.format_version(), FORMAT_VERSION_HEADED);
    let file = doc.to_canonical_json();

    history.undo(&mut doc).expect("the heading is live");
    assert_eq!(
        doc.to_canonical_json(),
        before,
        "byte for byte, stamp first"
    );
    history.redo(&mut doc).expect("the slot is free");
    assert_eq!(doc.to_canonical_json(), file);
}

/// And the edit answers the same way the file does, so the two doors into the
/// document cannot disagree about what a heading is.
#[test]
fn a_turn_outside_the_lap_is_refused_at_the_edit_too() {
    let mut doc = block::trousers();
    hang(&doc)
        .apply(&mut doc)
        .expect("both ends are nodes of the front");
    let key = only(&doc);
    let refused = Command::SetHangHeading {
        hang: key,
        to: Some(Heading::facing(-180.0, Sense::Leftward)),
    }
    .apply(&mut doc);
    assert_eq!(refused, Err(DocError::HangTurn));
    assert_eq!(
        doc.hangs.get(key).and_then(|held| held.heading),
        None,
        "and nothing was written"
    );
    assert_eq!(doc.format_version(), FORMAT_VERSION_HUNG);
    assert_eq!(
        doc.hangs.get(key).map(|held| held.station.as_str()),
        Some(Hang::WAIST)
    );
}
