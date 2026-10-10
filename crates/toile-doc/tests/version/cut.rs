use toile_doc::{
    Command, Doc, FORMAT_VERSION, FORMAT_VERSION_CUT, FormatError, Piece, PieceKey, block,
};

use super::dart::darted;
use super::elastic::elasticated;
use super::hang::hung;
use super::line::lined;
use super::placement::{FIRST_PIECE_END, placed};
use super::symmetry::folded;
use super::{EMPTY, SHIPPED, header, linked, restamped, rewritten, shaped};

/// The first piece of the shipped block saying how it is cut, in canonical
/// form: the four fields stand between the grain and where a placement goes.
const FIRST_PIECE_CUT: &str = "\"radians\": 1.5707963267948966
          },
          \"seam_allowance\": 1.5,
          \"quantity\": 2,
          \"letter\": \"A\",
          \"labels\": [
            \"DELANTERO\",
            \"cortar 2 espejadas\"
          ]
        }";

/// `file` with its first piece saying how it is cut, stamped `version`.
pub(super) fn cut(file: &str, version: u32) -> String {
    assert!(
        file.contains(FIRST_PIECE_END),
        "the block moved under the test"
    );
    restamped(&file.replacen(FIRST_PIECE_END, FIRST_PIECE_CUT, 1), version)
}

/// The front of the block, and the document it is drawn in.
fn front() -> (Doc, PieceKey) {
    let doc = block::trousers();
    let piece = doc.piece_named(block::FRONT).expect("the block draws one");
    (doc, piece)
}

/// Every file on disk was written before a piece could say how it is cut, and
/// a build that knows the four fields may not change a byte of one of them —
/// the stamp least of all. Serde defaults are what read them, and
/// `skip_serializing_if` is what keeps them out of the bytes on the way back.
#[test]
fn every_file_written_before_the_cut_re_saves_byte_identical() {
    assert_eq!(block::trousers().format_version(), FORMAT_VERSION);
    for file in [
        SHIPPED.to_owned(),
        EMPTY.to_owned(),
        shaped(2),
        linked(3, true),
        placed(SHIPPED, 4),
    ] {
        for key in ["seam_allowance", "quantity", "letter", "labels"] {
            assert!(!file.contains(key), "{key}: {file}");
        }
        assert_eq!(rewritten(&file).as_bytes(), file.as_bytes());
    }
}

/// A piece that says how it is cut takes the number after the dart's. Builds
/// that read version 9 predate all four fields at once: stamped 9, such a file
/// would open there with the allowance quietly gone, and whoever cut the piece
/// would cut it on the line it is sewn on.
#[test]
fn the_version_stamp_is_10_exactly_when_a_piece_says_how_it_is_cut() {
    for (file, stamp) in [
        (cut(SHIPPED, 1), 10),
        (cut(SHIPPED, 10), 10),
        (cut(&shaped(2), 2), 10),
        (cut(&linked(3, true), 3), 10),
        (restamped(SHIPPED, 10), 1),
        (restamped(&shaped(2), 10), 2),
        (restamped(&placed(SHIPPED, 4), 10), 4),
    ] {
        let doc = Doc::from_json(&file).unwrap_or_else(|error| panic!("{error}: {file}"));
        assert_eq!(doc.format_version(), stamp, "{file}");
        assert!(doc.to_canonical_json().starts_with(&header(stamp)));
    }
}

#[test]
fn a_version_10_document_round_trips_byte_identical() {
    for file in [cut(SHIPPED, 10), cut(&linked(3, true), 10)] {
        assert_eq!(rewritten(&file).as_bytes(), file.as_bytes());
        let doc = Doc::from_json(&file).expect("this build reads version 10");
        assert_eq!(Doc::from_json(&doc.to_canonical_json()), Ok(doc));
    }
}

/// One number for the four, and each of the four earns it on its own: a reader
/// that predates them loses whichever one it is handed, so the stamp cannot
/// wait for the others to arrive.
#[test]
fn any_one_of_the_four_fields_alone_asks_for_the_cut_s_version() {
    let edits: [fn(PieceKey) -> Command; 4] = [
        |piece| Command::SetSeamAllowance {
            piece,
            to: Some(1.5),
        },
        |piece| Command::SetQuantity { piece, to: 2 },
        |piece| Command::SetLetter {
            piece,
            to: Some("A".to_owned()),
        },
        |piece| Command::SetLabels {
            piece,
            to: vec!["cortar 2 espejadas".to_owned()],
        },
    ];
    for make in edits {
        let (mut doc, piece) = front();
        assert_eq!(doc.format_version(), FORMAT_VERSION);
        let edit = make(piece);
        edit.clone().apply(&mut doc).expect("the front is live");
        assert_eq!(doc.format_version(), FORMAT_VERSION_CUT, "{edit:?}");
        let written = doc.to_canonical_json();
        assert_eq!(Doc::from_json(&written), Ok(doc), "{written}");
    }
}

/// A count of one is the count a file leaves out, so writing it changes no
/// byte and asks for no version. Zero is not a count at all.
#[test]
fn the_count_a_file_leaves_out_is_one_and_zero_is_refused() {
    let (mut doc, piece) = front();
    let before = doc.to_canonical_json();
    Command::SetQuantity {
        piece,
        to: Piece::CUT_ONCE,
    }
    .apply(&mut doc)
    .expect("a piece cut once is a piece cut once");
    assert_eq!(doc.to_canonical_json().as_bytes(), before.as_bytes());
    assert_eq!(doc.format_version(), FORMAT_VERSION);
}

/// A build cannot know what a later field means, so it refuses the file for its
/// version rather than open it and drop that field on the next save.
#[test]
fn a_version_11_document_is_refused_loudly() {
    for later in [
        restamped(SHIPPED, 11),
        restamped(&shaped(2), 11),
        placed(SHIPPED, 11),
        elasticated(11),
        lined(11),
        folded(11),
        hung(11),
        darted(11),
        cut(SHIPPED, 11),
    ] {
        let error = Doc::from_json(&later).expect_err("this build reads up to version 10");
        assert_eq!(
            error,
            FormatError::UnknownVersion {
                found: 11,
                newest: FORMAT_VERSION_CUT
            }
        );
        assert!(
            error
                .to_string()
                .contains("version 11; this build reads versions 1 to 10"),
            "{error}"
        );
    }
}

/// The file is the other way in, so the rule that refuses an allowance at the
/// edit has to meet one that arrives by hand.
#[test]
fn numbers_no_piece_is_cut_by_are_refused_when_the_file_carries_them() {
    let file = cut(SHIPPED, 10);
    for (from, to) in [
        ("\"seam_allowance\": 1.5", "\"seam_allowance\": -1.5"),
        ("\"quantity\": 2", "\"quantity\": 0"),
    ] {
        assert!(file.contains(from), "the fixture moved under the test");
        let broken = file.replacen(from, to, 1);
        let error = Doc::from_json(&broken).expect_err(to);
        assert!(matches!(error, FormatError::Cut(_)), "{to}: {error}");
    }
}
