use toile_doc::{BodyShape, Command, Doc, Identity, Snapshot, block};

use super::{ANA, STEM, ana, edited, retyped};

/// FNV-1a 64 of Ana's current session as the canonical writer spells what a
/// body copies from it, worked out apart from the code under test:
///
/// ```text
/// {
///   "values": {
///     "cadera": 95.5,
///     "cintura": 71.25,
///     "tiro": 27
///   },
///   "phenotype": {
///     "sex": 1,
///     "age_years": 34,
///     "build": 0.62,
///     "muscle": 0.4,
///     "proportions": 0.25
///   }
/// }
/// ```
const CURRENT: &str = "ee0e7e62877ed721";

fn current(persona: &toile_doc::Persona) -> Snapshot {
    persona.current().cloned().expect("Ana has sessions")
}

#[test]
fn the_fingerprint_is_fnv_1a_of_the_canonical_bytes_of_what_is_copied() {
    assert_eq!(current(&ana()).fingerprint(), CURRENT);
}

/// Nothing but what a body copies may move the fingerprint, or the band
/// would rise for a note, a new name or a file saved by another tool.
#[test]
fn the_fingerprint_ignores_what_is_not_copied_and_how_the_file_is_spelled() {
    let retyped = toile_doc::Persona::from_json(&retyped()).expect("the same person");
    let variants = [
        retyped,
        edited("\"name\": \"Ana\"", "\"name\": \"Ana María\""),
        edited("prefiere el tiro alto", "prefiere el tiro bajo"),
        edited("\"date\": \"2026-09-01\"", "\"date\": \"2026-09-02\""),
        edited("\"estatura\": 165", "\"estatura\": 166"),
    ];
    for persona in variants {
        assert_eq!(current(&persona).fingerprint(), CURRENT, "{persona:?}");
    }

    let mut doc = block::trousers();
    let mannequin = ana().to_mannequin(STEM).expect("Ana has a session");
    Command::AddMannequin {
        identity: Identity::New,
        mannequin,
    }
    .apply(&mut doc)
    .expect("no body of the block is called Ana");
    let key = doc.mannequin_named("Ana").expect("she was just added");
    Command::RenameMannequin {
        mannequin: key,
        to: "Ana, prueba 2".to_owned(),
    }
    .apply(&mut doc)
    .expect("the name is free");
    let reopened = Doc::from_json(&doc.to_canonical_json()).expect("the document reads");
    let body = reopened.mannequins.get(key).expect("the body survives");
    let copy = Snapshot {
        date: "1999-01-01".to_owned(),
        values: body.values.clone(),
        phenotype: body.phenotype,
    };
    assert_eq!(copy.fingerprint(), CURRENT);
    let origin = body.origin.as_ref().expect("the link survives");
    assert_eq!(origin.fnv, CURRENT);
    assert!(!ana().differs_from(origin));
}

#[test]
fn the_fingerprint_moves_with_every_value_and_every_scale() {
    let session = current(&ana());
    let up = |value: f64| f64::from_bits(value.to_bits() + 1);
    for name in session.values.keys() {
        let mut moved = session.clone();
        let value = moved.values.get_mut(name).expect("the key was just read");
        *value = up(*value);
        assert_ne!(moved.fingerprint(), CURRENT, "{name}");
    }

    let shape = session.phenotype.expect("the current session is shaped");
    let shapes = [
        BodyShape {
            sex: up(shape.sex),
            ..shape
        },
        BodyShape {
            age_years: up(shape.age_years),
            ..shape
        },
        BodyShape {
            build: up(shape.build),
            ..shape
        },
        BodyShape {
            muscle: up(shape.muscle),
            ..shape
        },
        BodyShape {
            proportions: up(shape.proportions),
            ..shape
        },
    ];
    for moved in shapes {
        let mut reshaped = session.clone();
        reshaped.phenotype = Some(moved);
        assert_ne!(reshaped.fingerprint(), CURRENT, "{moved:?}");
    }

    let mut unshaped = session.clone();
    unshaped.phenotype = None;
    let mut renamed = session.clone();
    let tiro = renamed.values.remove("tiro").expect("Ana carries tiro");
    renamed.values.insert("tiro_total".to_owned(), tiro);
    let mut grown = session;
    grown.values.insert("muslo".to_owned(), 55.0);
    for changed in [unshaped, renamed, grown] {
        assert_ne!(changed.fingerprint(), CURRENT, "{changed:?}");
    }
}

#[test]
fn the_band_is_raised_by_the_tape_or_the_shape_and_by_nothing_else() {
    let origin = ana()
        .to_mannequin(STEM)
        .expect("Ana has a session")
        .origin
        .expect("a copy is stamped");
    assert!(!ana().differs_from(&origin));
    assert!(edited("\"cintura\": 71.25", "\"cintura\": 71.3").differs_from(&origin));
    assert!(edited("\"build\": 0.62", "\"build\": 0.63").differs_from(&origin));
    assert!(!edited("tiro alto", "tiro bajo").differs_from(&origin));

    let mut saved_again = ana();
    let mut same_tape = current(&saved_again);
    same_tape.date = "2026-10-01".to_owned();
    saved_again.taken.push(same_tape);
    assert!(!saved_again.differs_from(&origin));
    assert!(ANA.contains("\"toile_persona\": 1"));
}
