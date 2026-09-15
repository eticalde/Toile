use toile_doc::{
    Command, Doc, DocError, FormatError, Identity, Origin, PERSONA_VERSION, Persona, PersonaError,
    block,
};

use super::{ANA, PRIVATE, STEM, ana, retyped};

/// One person is one file with exactly one text, so that correcting a single
/// measurement is a single line of a diff.
#[test]
fn a_persona_file_round_trips_byte_identical() {
    let persona = ana();
    assert_eq!(persona.taken.len(), 2);
    let written = persona.to_canonical_json().expect("Ana is writable");
    assert_eq!(written.as_bytes(), ANA.as_bytes());

    let typed = Persona::from_json(&retyped()).expect("the same person, spelled otherwise");
    assert_eq!(typed, persona);
    let canonical = typed.to_canonical_json().expect("Ana is writable");
    assert_eq!(canonical.as_bytes(), ANA.as_bytes());
}

/// A file from a later Toile is refused for its version, before this build
/// tries to understand a shape it may not know.
#[test]
fn an_unknown_persona_version_is_refused_loudly() {
    let later = ANA.replacen("\"toile_persona\": 1", "\"toile_persona\": 2", 1);
    let error = Persona::from_json(&later).expect_err("no build reads version 2 yet");
    assert_eq!(
        error,
        PersonaError::UnknownVersion {
            found: 2,
            newest: PERSONA_VERSION,
        }
    );
    assert!(error.to_string().contains("version 2"), "{error}");

    let garbled = "{\"taken\": \"not a list\", \"toile_persona\": 7}";
    assert!(matches!(
        Persona::from_json(garbled),
        Err(PersonaError::UnknownVersion { found: 7, .. })
    ));
}

#[test]
fn a_pattern_is_not_a_person_and_a_person_is_not_a_pattern() {
    let pattern = block::trousers().to_canonical_json();
    assert_eq!(Persona::from_json(&pattern), Err(PersonaError::NoHeader));
    assert_eq!(Doc::from_json(ANA), Err(FormatError::NoHeader));
}

#[test]
fn a_file_that_breaks_what_a_person_promises_is_refused() {
    let nobody = "{\"toile_persona\": 1, \"name\": \"Ana\", \"notes\": \"\", \"taken\": []}";
    assert_eq!(Persona::from_json(nobody), Err(PersonaError::NothingTaken));

    let reversed = ANA.replacen("2026-03-02", "2026-10-02", 1);
    assert!(matches!(
        Persona::from_json(&reversed),
        Err(PersonaError::OutOfOrder { .. })
    ));
    let same_day = ANA.replacen("2026-03-02", "2026-09-01", 1);
    assert!(Persona::from_json(&same_day).is_ok());

    let undated = ANA.replacen("2026-03-02", "2 de marzo", 1);
    assert_eq!(
        Persona::from_json(&undated),
        Err(PersonaError::Invalid(DocError::NotADay(
            "2 de marzo".to_owned()
        )))
    );
    let unspellable = ANA.replacen("\"tiro\": 27", "\"tiro\": null", 1);
    assert!(matches!(
        Persona::from_json(&unspellable),
        Err(PersonaError::Malformed(_))
    ));
    assert!(matches!(
        Persona::from_json(&ANA[..ANA.len() / 2]),
        Err(PersonaError::Truncated(_))
    ));
}

/// The library file is the person at rest: a save that wrote something the
/// reader refuses would lose her.
#[test]
fn a_person_the_reader_would_refuse_is_never_written() {
    let mut persona = ana();
    persona.taken[1]
        .values
        .insert("cintura".to_owned(), f64::NAN);
    assert_eq!(
        persona.to_canonical_json(),
        Err(PersonaError::Invalid(DocError::NonFinite(
            "cintura".to_owned()
        )))
    );
    persona.taken.clear();
    assert_eq!(persona.to_canonical_json(), Err(PersonaError::NothingTaken));
}

#[test]
fn using_a_person_copies_the_current_session_and_stamps_where_it_came_from() {
    let persona = ana();
    let current = persona.current().expect("Ana has sessions");
    assert_eq!(current.date, "2026-09-01");
    let set = persona
        .to_mannequin(STEM)
        .expect("Ana has a session and a stem");
    assert_eq!(set.name, "Ana");
    assert_eq!(set.values, current.values);
    assert_eq!(set.phenotype, current.phenotype);
    assert_eq!(
        set.origin,
        Some(Origin {
            persona: STEM.to_owned(),
            taken: "2026-09-01".to_owned(),
            fnv: current.fingerprint(),
        })
    );

    assert_eq!(
        persona.to_mannequin("Ana"),
        Err(PersonaError::Invalid(DocError::NotAStem("Ana".to_owned())))
    );
    let nobody = Persona {
        taken: Vec::new(),
        ..persona
    };
    assert_eq!(nobody.to_mannequin(STEM), Err(PersonaError::NothingTaken));
}

#[test]
fn the_notes_never_reach_a_document() {
    let persona = ana();
    assert!(persona.notes.starts_with(PRIVATE));
    let mut doc = block::trousers();
    let mannequin = persona.to_mannequin(STEM).expect("Ana has a session");
    let refresh = persona
        .refresh(STEM, doc.resolve_with)
        .expect("Ana has a session");
    Command::AddMannequin {
        identity: Identity::New,
        mannequin,
    }
    .apply(&mut doc)
    .expect("no body of the block is called Ana");
    refresh.clone().apply(&mut doc).expect("the body is live");
    assert!(!doc.to_canonical_json().contains(PRIVATE));
    assert!(!format!("{refresh:?}").contains(PRIVATE));
}
