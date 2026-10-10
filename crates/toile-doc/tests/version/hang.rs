use toile_doc::{
    Command, Doc, DocError, EdgeRange, FORMAT_VERSION_HUNG, Hang, History, Identity, block,
};

use super::elastic::elasticated;
use super::placement::placed;
use super::{EMPTY, SHIPPED, header, linked, restamped, rewritten, shaped};

/// The edit that hangs the shipped block's waistline from the body's waist.
pub(super) fn hang(doc: &Doc) -> Command {
    let front = doc.piece_named(block::FRONT).expect("the block draws one");
    let ends = ["cintura_cf", "cintura_lat"]
        .map(|label| doc.shows_label(front, label).expect("the block names it"));
    Command::AddHang {
        identity: Identity::New,
        hang: Hang::new(EdgeRange::between(front, ends[0], ends[1]), Hang::WAIST),
    }
}

/// The shipped block hung from the waist, stamped `version`.
pub(super) fn hung(version: u32) -> String {
    let mut doc = block::trousers();
    hang(&doc)
        .apply(&mut doc)
        .expect("both ends are nodes of the front");
    restamped(&doc.to_canonical_json(), version)
}

/// Every file on disk was written before a garment could be hung from the body,
/// and reading one in a build that knows the hang may not change a byte of it,
/// the stamp least of all.
#[test]
fn every_file_written_before_the_hang_re_saves_byte_identical() {
    for file in [
        SHIPPED.to_owned(),
        EMPTY.to_owned(),
        shaped(2),
        linked(3, false),
        linked(3, true),
        placed(SHIPPED, 4),
        elasticated(5),
    ] {
        assert!(!file.contains("hangs"), "{file}");
        assert_eq!(rewritten(&file).as_bytes(), file.as_bytes());
    }
}

/// A hang takes the number after the axis's. Builds that read version 7 predate
/// it: stamped 7, a product would open there with nothing saying where on the
/// body the garment belongs, drape into a garment that slides off, and saving
/// would make that for good.
#[test]
fn the_version_stamp_is_8_exactly_when_the_document_hangs_from_the_body() {
    for (file, stamp) in [
        (hung(1), 8),
        (hung(7), 8),
        (hung(8), 8),
        (restamped(SHIPPED, 8), 1),
        (restamped(&shaped(2), 8), 2),
        (restamped(&linked(3, false), 8), 3),
        (restamped(&placed(SHIPPED, 4), 8), 4),
        (restamped(&elasticated(5), 8), 5),
    ] {
        let doc = Doc::from_json(&file).unwrap_or_else(|error| panic!("{error}: {file}"));
        assert_eq!(doc.format_version(), stamp, "{file}");
        assert!(doc.to_canonical_json().starts_with(&header(stamp)));
    }
}

#[test]
fn a_version_8_document_round_trips_byte_identical() {
    let file = hung(8);
    assert!(file.contains("\"station\": \"cintura\""), "{file}");
    assert_eq!(rewritten(&file).as_bytes(), file.as_bytes());
    let doc = Doc::from_json(&file).expect("this build reads version 8");
    assert_eq!(Doc::from_json(&doc.to_canonical_json()), Ok(doc));
}

/// And a version 7 document still reads, which is the other half of extending
/// the format rather than migrating it.
#[test]
fn a_version_7_document_still_reads_and_hangs_from_nothing() {
    let file = restamped(SHIPPED, 7);
    let doc = Doc::from_json(&file).expect("this build still reads version 7");
    assert!(doc.hangs.is_empty());
    assert_eq!(doc, block::trousers());
}

/// A station hand-typed into the file is refused at the door, for the reason a
/// hand-typed elastic ratio is: it reaches the placement as a height to hold
/// the cloth at, and a name with no ring behind it has none.
#[test]
fn a_station_the_file_carries_is_checked_before_the_pattern_opens() {
    let file = hung(8);
    for station in ["\"cintura_lat\"", "\"estatura\"", "\"waist\"", "\"\""] {
        let edited = file.replacen("\"cintura\"", station, 1);
        assert_ne!(edited, file, "{station} left the file as it was");
        let error = Doc::from_json(&edited).expect_err(station);
        assert!(error.to_string().contains("girths"), "{station}: {error}");
    }
}

/// The stamp follows the content through an undo as well: a product whose only
/// hang has been taken back off is the file it was before, stamp and all.
#[test]
fn a_hang_put_on_and_undone_takes_the_file_to_version_8_and_back() {
    let mut doc = block::trousers();
    let before = doc.to_canonical_json();
    let mut history = History::new();
    let edit = hang(&doc);
    history
        .edit(&mut doc, edit)
        .expect("both ends are nodes of the front");
    assert_eq!(doc.format_version(), FORMAT_VERSION_HUNG);
    let file = doc.to_canonical_json();

    history.undo(&mut doc).expect("the hang is live");
    assert_eq!(
        doc.to_canonical_json(),
        before,
        "byte for byte, stamp first"
    );
    history.redo(&mut doc).expect("the slot is free");
    assert_eq!(doc.to_canonical_json(), file);
}

/// A station the body has no ring for is refused at the edit too, so the two
/// doors into the document answer the same way.
#[test]
fn a_station_no_body_carries_a_ring_for_is_refused_at_the_edit() {
    let mut doc = block::trousers();
    let Command::AddHang { hang, .. } = hang(&doc) else {
        panic!("the fixture writes one hang");
    };
    let refused = Command::AddHang {
        identity: Identity::New,
        hang: Hang::new(hang.at, "estatura"),
    }
    .apply(&mut doc);
    assert_eq!(refused, Err(DocError::HangStation("estatura".to_owned())));
    assert!(doc.hangs.is_empty(), "and nothing was written");
}
