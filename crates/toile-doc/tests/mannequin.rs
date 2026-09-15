#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_doc::{
    Applied, BodyShape, ChangeClass, Command, Doc, DocError, FORMAT_VERSION,
    FORMAT_VERSION_EXTENDED, History, Identity, MannequinKey, MeasureSet, block,
};

fn talla(doc: &Doc) -> MannequinKey {
    doc.mannequin_named("Talla 42")
        .expect("the block carries it")
}

fn ana() -> MeasureSet {
    MeasureSet::new("Ana", [("cintura", 72.0), ("cadera", 96.0)]).shaped(BodyShape {
        sex: 1.0,
        age_years: 34.0,
        ..BodyShape::default()
    })
}

fn add_ana() -> Command {
    Command::AddMannequin {
        identity: Identity::New,
        mannequin: ana(),
    }
}

/// The mannequin edits whose inverse gives back the very document, each one
/// changing something. An addition is not among them: its undo leaves the
/// slot it opened behind, as every addition does.
fn reversible(doc: &Doc) -> Vec<Command> {
    let other = talla(doc);
    vec![
        Command::SetPhenotype {
            mannequin: other,
            to: Some(BodyShape::default()),
        },
        Command::RemoveMannequin { mannequin: other },
        Command::RenameMannequin {
            mannequin: other,
            to: "Talla 44".to_owned(),
        },
    ]
}

#[test]
fn apply_then_invert_gives_back_the_same_bytes() {
    let original = block::trousers();
    for command in reversible(&original) {
        let mut doc = original.clone();
        let Applied { inverse, class, .. } = command
            .clone()
            .apply(&mut doc)
            .unwrap_or_else(|error| panic!("{command:?} applies: {error}"));
        assert_ne!(doc, original, "{command:?} changed nothing");
        assert_eq!(class, command.class(), "{command:?}");
        inverse
            .clone()
            .apply(&mut doc)
            .unwrap_or_else(|error| panic!("{inverse:?} applies: {error}"));
        assert_eq!(doc, original, "{inverse:?} did not undo {command:?}");
    }
}

#[test]
fn a_phenotype_is_work_for_the_drape_and_the_rest_costs_nothing() {
    let doc = block::trousers();
    let mut all = reversible(&doc);
    all.push(add_ana());
    for command in all {
        let expected = match command {
            Command::SetPhenotype { .. } => ChangeClass::Sim,
            _ => ChangeClass::Metadata,
        };
        assert_eq!(command.class(), expected, "{command:?}");
    }
}

/// Shaping a body is what takes a file to version 2, and undoing it takes the
/// file back to the very bytes it had.
#[test]
fn a_phenotype_set_and_undone_takes_the_file_to_version_2_and_back() {
    let mut doc = block::trousers();
    let before = doc.to_canonical_json();
    let mut history = History::new();
    let shape = Command::SetPhenotype {
        mannequin: talla(&doc),
        to: Some(BodyShape::default()),
    };
    let applied = history.edit(&mut doc, shape).expect("the body is live");
    assert!(applied.touched.is_empty());
    assert_eq!(doc.format_version(), FORMAT_VERSION_EXTENDED);
    let file = doc.to_canonical_json();
    assert!(file.contains("\"age_years\": 25,"), "{file}");
    assert_eq!(Doc::from_json(&file), Ok(doc.clone()));

    history.undo(&mut doc).expect("the body is live");
    assert_eq!(doc.format_version(), FORMAT_VERSION);
    assert_eq!(doc.to_canonical_json(), before);
    history.redo(&mut doc).expect("the body is live");
    assert_eq!(doc.to_canonical_json(), file);
}

#[test]
fn an_added_body_undone_is_gone_and_redone_under_the_same_key() {
    let mut doc = block::trousers();
    let mut history = History::new();
    let applied = history.edit(&mut doc, add_ana()).expect("the name is free");
    let added = doc.mannequin_named("Ana").expect("it was added");
    assert_eq!(
        applied.inverse,
        Command::RemoveMannequin { mannequin: added }
    );
    assert!(applied.touched.is_empty());

    history.undo(&mut doc).expect("nothing resolves against it");
    assert_eq!(doc.mannequins.get(added), None);
    assert_eq!(doc.format_version(), FORMAT_VERSION);
    history.redo(&mut doc).expect("its slot is still open");
    assert_eq!(doc.mannequins.get(added), Some(&ana()));
}

/// Choosing a body inside the gesture that added it is the order a fold could
/// break: folded onto the gesture's earlier choice, the choice would be
/// replayed before the body existed.
#[test]
fn undoing_the_addition_of_the_body_resolved_against_leaves_no_dangling_key() {
    let mut doc = block::trousers();
    let original = doc.resolve_with;
    let other = talla(&doc);
    let mut history = History::new();
    history.begin("usar maniquí");
    history
        .edit(&mut doc, Command::ResolveWith { mannequin: other })
        .expect("the body is live");
    history.edit(&mut doc, add_ana()).expect("the name is free");
    let added = doc.mannequin_named("Ana").expect("it was added");
    history
        .edit(&mut doc, Command::ResolveWith { mannequin: added })
        .expect("the body is live");
    history.end();

    history
        .undo(&mut doc)
        .expect("the choice goes back before the body");
    assert_eq!(doc.resolve_with, original);
    assert_eq!(doc.mannequins.get(added), None);
    assert!(Doc::from_json(&doc.to_canonical_json()).is_ok());

    history
        .redo(&mut doc)
        .expect("the body comes back before the choice");
    assert_eq!(doc.resolve_with, added);
    assert_eq!(doc.measures(), Some(&ana()));
}

#[test]
fn the_body_the_pattern_resolves_against_cannot_be_removed() {
    let mut doc = block::trousers();
    let before = doc.clone();
    let refused = Command::RemoveMannequin {
        mannequin: doc.resolve_with,
    }
    .apply(&mut doc);
    assert_eq!(refused, Err(DocError::BodyInUse("Etienne".to_owned())));
    assert_eq!(doc, before);
}

#[test]
fn a_name_another_body_carries_is_refused() {
    let mut doc = block::trousers();
    let other = talla(&doc);
    let taken = |name: &str| Err(DocError::DuplicateMannequinName(name.to_owned()));
    let rename = |to: &str| Command::RenameMannequin {
        mannequin: other,
        to: to.to_owned(),
    };
    assert_eq!(rename("Etienne").apply(&mut doc), taken("Etienne"));
    let twin = Command::AddMannequin {
        identity: Identity::New,
        mannequin: MeasureSet::new("Talla 42", []),
    };
    assert_eq!(twin.apply(&mut doc), taken("Talla 42"));
    assert!(rename("Talla 42").apply(&mut doc).is_ok());
    let stale = Command::SetPhenotype {
        mannequin: MannequinKey::new(9, 0),
        to: None,
    };
    assert!(matches!(
        stale.apply(&mut doc),
        Err(DocError::StaleKey { .. })
    ));
}

/// Two bodies trading names pass through a moment where both carry one, and
/// the undo of that gesture has to get through it in both directions.
#[test]
fn two_bodies_that_trade_names_undo_and_redo() {
    let mut doc = block::trousers();
    let (first, second) = (doc.resolve_with, talla(&doc));
    let mut history = History::new();
    history.begin("renombrar");
    for (mannequin, to) in [(first, "tmp"), (second, "Etienne"), (first, "Talla 42")] {
        let rename = Command::RenameMannequin {
            mannequin,
            to: to.to_owned(),
        };
        history.edit(&mut doc, rename).expect("the name is free");
    }
    history.end();
    let traded = doc.to_canonical_json();

    history
        .undo(&mut doc)
        .expect("a replay lets the collision by");
    assert_eq!(
        doc.to_canonical_json(),
        block::trousers().to_canonical_json()
    );
    history
        .redo(&mut doc)
        .expect("a replay lets the collision by");
    assert_eq!(doc.to_canonical_json(), traded);
}

/// A value the file cannot spell is refused by the command, before any writer
/// sees it: autosaved, it would be written as `null` and never reopen.
#[test]
fn a_value_the_file_cannot_spell_is_refused_before_it_is_written() {
    let mut doc = block::trousers();
    let before = doc.to_canonical_json();
    let body = doc.resolve_with;
    let mut history = History::new();
    let measure = Command::SetMeasure {
        mannequin: body,
        name: "cintura".to_owned(),
        to: f64::NAN,
    };
    assert!(matches!(
        history.edit(&mut doc, measure),
        Err(DocError::NonFinite(_))
    ));
    let shape = Command::SetPhenotype {
        mannequin: body,
        to: Some(BodyShape {
            age_years: f64::INFINITY,
            ..BodyShape::default()
        }),
    };
    assert!(matches!(
        history.edit(&mut doc, shape),
        Err(DocError::NonFinite(_))
    ));
    assert_eq!(doc.to_canonical_json(), before);
}
