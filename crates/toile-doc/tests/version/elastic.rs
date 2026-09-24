use toile_doc::{
    Command, Doc, EdgeRange, Elastic, FORMAT_VERSION_ELASTIC, History, Identity, block,
};

use super::placement::placed;
use super::{EMPTY, SHIPPED, header, linked, restamped, rewritten, shaped};

/// The edit that puts an elastic across the waist of the shipped block's front.
fn band(doc: &Doc) -> Command {
    let front = doc.piece_named(block::FRONT).expect("the block draws one");
    let ends = ["cintura_cf", "cintura_lat"]
        .map(|label| doc.shows_label(front, label).expect("the block names it"));
    Command::AddElastic {
        identity: Identity::New,
        elastic: Elastic::new(EdgeRange::between(front, ends[0], ends[1]), 0.85, 30.0),
    }
}

/// The shipped block with that elastic on it, stamped `version`.
pub(super) fn elasticated(version: u32) -> String {
    let mut doc = block::trousers();
    band(&doc)
        .apply(&mut doc)
        .expect("both ends are nodes of the front");
    restamped(&doc.to_canonical_json(), version)
}

/// Every file on disk was written before a stretch of contour could be held in,
/// and reading one in a build that knows the elastic may not change a byte of
/// it, the stamp least of all.
#[test]
fn every_file_written_before_the_elastic_re_saves_byte_identical() {
    for file in [
        SHIPPED.to_owned(),
        EMPTY.to_owned(),
        shaped(2),
        linked(3, false),
        linked(3, true),
        placed(SHIPPED, 4),
    ] {
        assert!(!file.contains("elastic"), "{file}");
        assert_eq!(rewritten(&file).as_bytes(), file.as_bytes());
    }
}

/// An elastic takes the number after the placement's. Builds that read version
/// 4 predate it: stamped 4, a product would open there with nothing holding it
/// on the body, drape into another garment, and saving would make that for
/// good.
#[test]
fn the_version_stamp_is_5_exactly_when_the_document_holds_an_elastic() {
    for (file, stamp) in [
        (elasticated(1), 5),
        (elasticated(4), 5),
        (elasticated(5), 5),
        (restamped(SHIPPED, 5), 1),
        (restamped(&shaped(2), 5), 2),
        (restamped(&linked(3, false), 5), 3),
        (restamped(&placed(SHIPPED, 4), 5), 4),
    ] {
        let doc = Doc::from_json(&file).unwrap_or_else(|error| panic!("{error}: {file}"));
        assert_eq!(doc.format_version(), stamp, "{file}");
        assert!(doc.to_canonical_json().starts_with(&header(stamp)));
    }
}

#[test]
fn a_version_5_document_round_trips_byte_identical() {
    let file = elasticated(5);
    assert!(file.contains("\"ratio\": 0.85,"), "{file}");
    assert!(file.contains("\"strength\": 30"), "{file}");
    assert_eq!(rewritten(&file).as_bytes(), file.as_bytes());
    let doc = Doc::from_json(&file).expect("this build reads version 5");
    assert_eq!(Doc::from_json(&doc.to_canonical_json()), Ok(doc));
}

/// The stamp follows the content through an undo as well: a product whose only
/// elastic has been taken back off is the file it was before, stamp and all.
#[test]
fn an_elastic_put_on_and_undone_takes_the_file_to_version_5_and_back() {
    let mut doc = block::trousers();
    let before = doc.to_canonical_json();
    let mut history = History::new();
    let edit = band(&doc);
    history
        .edit(&mut doc, edit)
        .expect("both ends are nodes of the front");
    assert_eq!(doc.format_version(), FORMAT_VERSION_ELASTIC);
    let file = doc.to_canonical_json();

    history.undo(&mut doc).expect("the elastic is live");
    assert_eq!(
        doc.to_canonical_json(),
        before,
        "byte for byte, stamp first"
    );
    history.redo(&mut doc).expect("the slot is free");
    assert_eq!(doc.to_canonical_json(), file);
}
