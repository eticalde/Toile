use toile_doc::{
    BodyShape, ChangeClass, Coalesced, Command, Doc, DocError, FORMAT_VERSION,
    FORMAT_VERSION_LINKED, History, Identity, MannequinKey, Origin, PersonaError, block,
};

use super::{STEM, ana, edited};

/// The trousers with Ana copied in from the library, and the key she took.
fn with_ana() -> (Doc, MannequinKey) {
    let mut doc = block::trousers();
    let mannequin = ana()
        .to_mannequin(STEM)
        .expect("Ana has a session and a stem");
    Command::AddMannequin {
        identity: Identity::New,
        mannequin,
    }
    .apply(&mut doc)
    .expect("no body of the block is called Ana");
    let key = doc.mannequin_named("Ana").expect("she was just added");
    (doc, key)
}

fn origin(doc: &Doc, key: MannequinKey) -> Origin {
    doc.mannequins
        .get(key)
        .and_then(|set| set.origin.clone())
        .expect("the body is linked to the library")
}

/// Ana corrected in the library: a tape that lost `tiro`, gained `muslo`
/// and moved `cintura`.
fn corrected() -> toile_doc::Persona {
    edited(
        "\"cintura\": 71.25,\n        \"tiro\": 27",
        "\"cintura\": 70.5,\n        \"muslo\": 55",
    )
}

#[test]
fn a_refresh_takes_the_current_session_and_undo_gives_back_the_bytes() {
    let (mut doc, key) = with_ana();
    let before = doc.to_canonical_json();
    let newer = corrected();
    assert!(newer.differs_from(&origin(&doc, key)));

    let mut history = History::new();
    let command = newer.refresh(STEM, key).expect("Ana has a session");
    let applied = history
        .edit(&mut doc, command)
        .expect("the body is live and the tape finite");
    assert!(applied.touched.is_empty(), "the pattern reads another body");
    let body = doc.mannequins.get(key).expect("a refresh keeps the body");
    let current = newer.current().expect("Ana has sessions");
    assert_eq!(body.values, current.values);
    assert_eq!(body.phenotype, current.phenotype);
    assert_eq!(body.name, "Ana");
    assert!(!newer.differs_from(&origin(&doc, key)));

    let after = doc.to_canonical_json();
    history.undo(&mut doc).expect("the body is live");
    assert_eq!(doc.to_canonical_json(), before);
    history.redo(&mut doc).expect("the body is live");
    assert_eq!(doc.to_canonical_json(), after);
}

/// The inverse is the same command carrying what was there, and what was
/// there may be a body the library never saw: no origin, no shape.
#[test]
fn refreshing_the_body_resolved_against_reshapes_every_piece_and_inverts_exactly() {
    let original = block::trousers();
    let mut doc = original.clone();
    let body = doc.resolve_with;
    let command = ana().refresh(STEM, body).expect("Ana has a session");
    assert_eq!(command.class(), ChangeClass::Shape);

    let applied = command.apply(&mut doc).expect("the body is live");
    assert_eq!(applied.class, ChangeClass::Shape);
    assert_eq!(applied.touched, doc.piece_keys());
    assert_eq!(doc.resolve_with, body);
    assert_eq!(doc.format_version(), FORMAT_VERSION_LINKED);
    assert!(matches!(
        &applied.inverse,
        Command::RefreshMannequin {
            phenotype: None,
            origin: None,
            ..
        }
    ));

    applied.inverse.apply(&mut doc).expect("the body is live");
    assert_eq!(doc, original);
    assert_eq!(doc.format_version(), FORMAT_VERSION);
    assert_eq!(doc.to_canonical_json(), original.to_canonical_json());
}

#[test]
fn a_refresh_the_file_could_not_hold_is_refused_and_changes_nothing() {
    let (mut doc, key) = with_ana();
    let before = doc.to_canonical_json();
    let Command::RefreshMannequin {
        values,
        phenotype,
        origin,
        ..
    } = corrected().refresh(STEM, key).expect("Ana has a session")
    else {
        panic!("a refresh builds a refresh");
    };
    let origin = origin.expect("a refresh from the library links the body");
    let refresh = |values, phenotype, origin| Command::RefreshMannequin {
        mannequin: key,
        values,
        phenotype,
        origin: Some(origin),
    };
    let mut unspellable = values.clone();
    unspellable.insert("cintura".to_owned(), f64::NAN);
    let heavy = phenotype.map(|shape| BodyShape {
        build: f64::INFINITY,
        ..shape
    });
    let link = |edit: fn(&mut Origin)| {
        let mut link = origin.clone();
        edit(&mut link);
        link
    };
    let refusals = [
        (
            refresh(unspellable, phenotype, origin.clone()),
            DocError::NonFinite("cintura".to_owned()),
        ),
        (
            refresh(values.clone(), heavy, origin.clone()),
            DocError::NonFinite("phenotype".to_owned()),
        ),
        (
            refresh(
                values.clone(),
                phenotype,
                link(|o| o.persona = "../ana".to_owned()),
            ),
            DocError::NotAStem("../ana".to_owned()),
        ),
        (
            refresh(
                values.clone(),
                phenotype,
                link(|o| o.taken = "ayer".to_owned()),
            ),
            DocError::NotADay("ayer".to_owned()),
        ),
        (
            refresh(
                values.clone(),
                phenotype,
                link(|o| o.fnv = "0xABC".to_owned()),
            ),
            DocError::NotAFingerprint("0xABC".to_owned()),
        ),
    ];
    for (command, error) in refusals {
        assert_eq!(command.apply(&mut doc), Err(error));
        assert_eq!(doc.to_canonical_json(), before);
    }
    let stale = Command::RefreshMannequin {
        mannequin: MannequinKey::new(9, 0),
        values,
        phenotype,
        origin: Some(origin),
    };
    assert!(matches!(
        stale.apply(&mut doc),
        Err(DocError::StaleKey { .. })
    ));
    assert_eq!(
        ana().refresh("Ana María", key),
        Err(PersonaError::Invalid(DocError::NotAStem(
            "Ana María".to_owned()
        )))
    );
}

/// A refresh rewrites the measurements one `SetMeasure` writes at a time, so
/// the later write must not fold back past it: redo would replay it before
/// the refresh, and the refresh would win.
#[test]
fn a_measurement_written_after_a_refresh_does_not_fold_across_it() {
    let (mut doc, key) = with_ana();
    let before = doc.to_canonical_json();
    let refresh = corrected().refresh(STEM, key).expect("Ana has a session");
    assert_eq!(refresh.coalesce_onto(&refresh), Coalesced::Separate);
    let waist = |to| Command::SetMeasure {
        mannequin: key,
        name: "cintura".to_owned(),
        to,
    };

    let mut history = History::new();
    history.begin("actualizar");
    for command in [waist(60.0), refresh, waist(61.0)] {
        history
            .edit(&mut doc, command)
            .expect("the body carries cintura");
    }
    history.end();
    let after = doc.to_canonical_json();
    assert!(after.contains("\"cintura\": 61,"), "{after}");

    history.undo(&mut doc).expect("the body is live");
    assert_eq!(doc.to_canonical_json(), before);
    history.redo(&mut doc).expect("the body is live");
    assert_eq!(doc.to_canonical_json(), after);
}
