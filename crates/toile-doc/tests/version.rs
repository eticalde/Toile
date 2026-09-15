#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_doc::{Doc, FormatError, MeasureSet, block};

/// The base block as it ships, written before any body carried a phenotype.
const SHIPPED: &str = include_str!("../../../assets/pantalon-base.toile");

/// The smallest document there is, as the writer spelled it before any body
/// carried a phenotype.
const EMPTY: &str = r#"{
  "toile": 1,
  "doc": {
    "pieces": {
      "issued": 0,
      "entries": []
    },
    "points": {
      "issued": 0,
      "entries": []
    },
    "seams": {
      "issued": 0,
      "entries": []
    },
    "notches": {
      "issued": 0,
      "entries": []
    },
    "darts": {
      "issued": 0,
      "entries": []
    },
    "symmetries": {
      "issued": 0,
      "entries": []
    },
    "pins": {
      "issued": 0,
      "entries": []
    },
    "variables": {
      "issued": 2,
      "entries": [
        {
          "id": "0.0",
          "name": "tolerancia_costura",
          "value": 0.5
        },
        {
          "id": "1.0",
          "name": "tolerancia_ratio",
          "value": 0.05
        }
      ]
    },
    "mannequins": {
      "issued": 1,
      "entries": [
        {
          "id": "0.0",
          "name": "Etienne",
          "values": {
            "cadera": 98.5,
            "cintura": 84
          }
        }
      ]
    },
    "resolve_with": "0.0"
  }
}
"#;

/// The end of the second body of the shipped block, where a phenotype goes.
const SECOND_BODY_END: &str = "\"tobillo\": 25\n          }\n        }";

/// The same place with a phenotype under the tape, in canonical form.
const SECOND_BODY_SHAPED: &str = "\"tobillo\": 25
          },
          \"phenotype\": {
            \"sex\": 1,
            \"age_years\": 34,
            \"build\": 0.62,
            \"muscle\": 0.4,
            \"proportions\": 0.25
          }
        }";

fn header(version: u32) -> String {
    format!("{{\n  \"toile\": {version},\n  \"doc\": {{\n")
}

/// `text` with its header claiming `version` instead.
fn restamped(text: &str, version: u32) -> String {
    let body = text
        .strip_prefix(&header(1))
        .or_else(|| text.strip_prefix(&header(2)))
        .expect("the file opens with a header this build writes");
    format!("{}{body}", header(version))
}

/// The shipped block with a phenotype on its second body, stamped `version`.
fn shaped(version: u32) -> String {
    assert!(
        SHIPPED.contains(SECOND_BODY_END),
        "the block moved under the test"
    );
    restamped(
        &SHIPPED.replacen(SECOND_BODY_END, SECOND_BODY_SHAPED, 1),
        version,
    )
}

fn rewritten(text: &str) -> String {
    Doc::from_json(text)
        .unwrap_or_else(|error| panic!("the file reads: {error}"))
        .to_canonical_json()
}

/// The format is extended, never migrated: a document that uses nothing new
/// has to come out as the very bytes an older Toile wrote for it, or every
/// pattern on disk would show a diff the day it is reopened.
#[test]
fn a_v1_document_without_the_new_field_serialises_byte_identical_to_before() {
    assert_eq!(
        block::trousers().to_canonical_json().as_bytes(),
        SHIPPED.as_bytes()
    );
    assert_eq!(rewritten(SHIPPED).as_bytes(), SHIPPED.as_bytes());

    let empty = Doc::new(MeasureSet::new(
        "Etienne",
        [("cintura", 84.0), ("cadera", 98.5)],
    ));
    assert_eq!(empty.to_canonical_json().as_bytes(), EMPTY.as_bytes());
    assert_eq!(rewritten(EMPTY).as_bytes(), EMPTY.as_bytes());

    for doc in [block::trouser_front(), block::trousers()] {
        let text = doc.to_canonical_json();
        assert!(text.starts_with(&header(1)), "{text}");
        assert!(!text.contains("phenotype"), "{text}");
        assert_eq!(rewritten(&text).as_bytes(), text.as_bytes());
    }
}

/// The version is a pure function of the content, whatever the file claimed:
/// a reader older than the phenotype has to refuse a file that carries one,
/// and must never be handed one it would silently drop.
#[test]
fn the_version_stamp_is_2_exactly_when_a_new_field_is_present_and_1_otherwise() {
    assert!(rewritten(&shaped(2)).starts_with(&header(2)));
    assert!(rewritten(&shaped(1)).starts_with(&header(2)));
    assert!(rewritten(&restamped(SHIPPED, 2)).starts_with(&header(1)));
    assert!(rewritten(SHIPPED).starts_with(&header(1)));
}

#[test]
fn a_version_2_document_round_trips_and_version_3_is_rejected_loudly() {
    let file = shaped(2);
    let doc = Doc::from_json(&file).expect("this build reads version 2");
    assert_eq!(doc.to_canonical_json().as_bytes(), file.as_bytes());
    assert_eq!(Doc::from_json(&doc.to_canonical_json()), Ok(doc));

    for later in [restamped(&file, 3), restamped(SHIPPED, 3)] {
        let error = Doc::from_json(&later).expect_err("no build reads version 3 yet");
        assert!(
            matches!(error, FormatError::UnknownVersion { found: 3, .. }),
            "{error}"
        );
        assert!(error.to_string().contains("version 3"), "{error}");
    }
}
