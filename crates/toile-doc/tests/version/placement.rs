use toile_doc::Doc;

use super::{EMPTY, SHIPPED, header, linked, restamped, rewritten, shaped};

/// The end of the first piece of the shipped block, where a placement goes.
const FIRST_PIECE_END: &str = "\"radians\": 1.5707963267948966\n          }\n        }";

/// The same place with the piece arranged on the overview, in canonical form.
const FIRST_PIECE_PLACED: &str = "\"radians\": 1.5707963267948966
          },
          \"placement\": {
            \"x\": 42.5,
            \"y\": -3
          }
        }";

/// `file` with its first piece arranged on the overview, stamped `version`.
pub(super) fn placed(file: &str, version: u32) -> String {
    assert!(
        file.contains(FIRST_PIECE_END),
        "the block moved under the test"
    );
    restamped(
        &file.replacen(FIRST_PIECE_END, FIRST_PIECE_PLACED, 1),
        version,
    )
}

/// Every file on disk was written before a piece could be arranged, and
/// reading it in a build that knows the placement may not change a byte of
/// it, the stamp least of all.
#[test]
fn every_file_written_before_the_placement_re_saves_byte_identical() {
    for file in [
        SHIPPED.to_owned(),
        EMPTY.to_owned(),
        shaped(2),
        linked(3, false),
        linked(3, true),
    ] {
        assert!(!file.contains("placement"), "{file}");
        assert_eq!(rewritten(&file).as_bytes(), file.as_bytes());
    }
}

/// A placement takes the number after the link's. Builds that read version 3
/// predate it: stamped 3, an arranged product would open there with every
/// piece back where the overview first laid it, and saving would make that
/// for good.
#[test]
fn the_version_stamp_is_4_exactly_when_a_piece_carries_a_placement() {
    for (file, stamp) in [
        (placed(SHIPPED, 1), 4),
        (placed(SHIPPED, 4), 4),
        (placed(&shaped(2), 2), 4),
        (placed(&linked(3, true), 3), 4),
        (restamped(SHIPPED, 4), 1),
        (restamped(&shaped(2), 4), 2),
        (restamped(&linked(3, false), 4), 3),
    ] {
        let doc = Doc::from_json(&file).unwrap_or_else(|error| panic!("{error}: {file}"));
        assert_eq!(doc.format_version(), stamp, "{file}");
        assert!(doc.to_canonical_json().starts_with(&header(stamp)));
    }
}

#[test]
fn a_version_4_document_round_trips_byte_identical() {
    for file in [placed(SHIPPED, 4), placed(&linked(3, true), 4)] {
        assert_eq!(rewritten(&file).as_bytes(), file.as_bytes());
        let doc = Doc::from_json(&file).expect("this build reads version 4");
        assert_eq!(Doc::from_json(&doc.to_canonical_json()), Ok(doc));
    }
}
