use toile_engine::draft::{Command, Elastic, Hang, Identity, block};

use super::*;

/// The headless door reads a garment held on the body and says what holds it,
/// which is the whole of what a person outside the app can do with one today:
/// no gesture puts a hang on, and this is where they see it is there.
#[test]
fn a_pattern_held_on_the_body_says_what_holds_it() {
    let mut doc = block::trousers();
    let front = doc.piece_named(block::FRONT).expect("the block draws one");
    let ends = ["cintura_cf", "cintura_lat"]
        .map(|label| doc.shows_label(front, label).expect("the block names it"));
    let at = EdgeRange::between(front, ends[0], ends[1]);
    Command::AddElastic {
        identity: Identity::New,
        elastic: Elastic::new(at, 0.85, 10.0),
    }
    .apply(&mut doc)
    .expect("both ends are nodes of the front");
    Command::AddHang {
        identity: Identity::New,
        hang: Hang::new(at, Hang::WAIST),
    }
    .apply(&mut doc)
    .expect("both ends are nodes of the front");

    // Through the file and not from the document in hand: what a person
    // outside the app has is bytes on disk, and the version those bytes
    // carry is the one this build has to read back.
    let written = doc.to_canonical_json();
    assert!(written.starts_with("{\n  \"toile\": 8,"), "{written}");
    let reread = Doc::from_json(&written).expect("this build reads it");
    let draft = Draft::from_doc(reread).expect("the block resolves");
    assert_eq!(
        holding(&draft),
        [
            "elástico  «Delantero» cintura_cf → cintura_lat · 85 % · fuerza 10",
            "colgado   «Delantero» cintura_cf → cintura_lat · de «cintura»",
        ]
    );
}

/// And a pattern nothing holds on says so, rather than showing an empty
/// heading a reader has to guess at.
#[test]
fn a_pattern_nothing_holds_on_says_nothing_holds_it() {
    let draft = Draft::from_doc(block::trousers()).expect("the block resolves");
    assert!(holding(&draft).is_empty());
    assert_eq!(
        sujecion(&draft),
        [
            String::new(),
            "sujeción".to_owned(),
            "  nada: la prenda no se sujeta al cuerpo".to_owned(),
        ]
    );
}
