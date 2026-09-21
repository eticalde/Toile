use std::collections::BTreeMap;

use toile_engine::body::Collider;
use toile_engine::draft::{Doc, MeasureSet, Persona, Snapshot};
use toile_engine::session::Session;

use super::table;
use crate::library::Library;
use crate::library::shelf::Shelf;
use crate::library::tests::Scratch;

/// The stem Etienne is filed under.
const STEM: &str = "etienne";

/// What Etienne's notes say, which no product may repeat.
const NOTES: &str = "hombro derecho más bajo";

/// Etienne as he was measured in March.
fn etienne() -> Persona {
    Persona {
        name: "Etienne".to_owned(),
        notes: NOTES.to_owned(),
        taken: vec![Snapshot {
            date: "2026-03-02".to_owned(),
            values: BTreeMap::from([
                ("cadera".to_owned(), 98.0),
                ("cintura".to_owned(), 84.5),
                ("estatura".to_owned(), 178.0),
            ]),
            phenotype: None,
        }],
    }
}

/// A scratch library holding Etienne, and the app's read of it.
fn filed(test: &str) -> (Scratch, Shelf) {
    let scratch = Scratch::new(test);
    let stem = scratch
        .library()
        .create(&etienne())
        .expect("Etienne is filed");
    assert_eq!(stem, STEM);
    let shelf = Shelf::over(Some(scratch.library()));
    (scratch, shelf)
}

/// A new product's table for `own`, and the bodies its body was fitted to, in
/// the order they were asked for.
fn cut(own: Option<&str>, shelf: &Shelf) -> (Session, Vec<Option<MeasureSet>>) {
    let mut asked = Vec::new();
    let session = table(own, shelf, |doc| {
        asked.push(doc.and_then(Doc::measures).cloned());
        Collider::demo()
    });
    (session, asked)
}

/// The document on the table, as its file would read.
fn written(session: &Session) -> String {
    session
        .draft()
        .expect("a product is open")
        .doc()
        .to_canonical_json()
}

/// A new product as every installation made it before the preference existed.
fn blank() -> String {
    written(&Session::blank(Collider::demo()))
}

#[test]
fn a_new_product_is_cut_for_the_default_person_the_library_holds() {
    let (_scratch, shelf) = filed("fresh-own");
    let (session, asked) = cut(Some(STEM), &shelf);

    let person = etienne();
    let current = person.current().expect("Etienne has a session");
    let copy = person.to_mannequin(STEM).expect("Etienne links");
    let expected = Doc::new(copy.clone()).to_canonical_json();
    assert_eq!(
        written(&session),
        expected,
        "a blank product, with him on it"
    );
    let draft = session.draft().expect("a product is open");
    let body = draft.doc().measures().expect("it resolves against a body");
    assert_eq!(draft.doc().mannequins.len(), 1);
    assert_eq!(body.name, "Etienne");
    assert_eq!(body.values, current.values);
    let origin = body.origin.as_ref().expect("the copy is linked");
    assert_eq!(origin.persona, STEM);
    assert_eq!(origin.taken, "2026-03-02");
    assert_eq!(origin.fnv, current.fingerprint());
    assert_eq!(
        draft.env().value("cintura"),
        Some(84.5),
        "what formulas read"
    );
    assert!(!written(&session).contains(NOTES), "notes stay home");
    assert_eq!(session.revision(), 0);
    assert!(!session.can_undo(), "a new product has no history");
    assert_eq!(asked, [Some(copy)], "the body is fitted to him");
}

#[test]
fn with_no_default_person_a_new_product_is_the_blank_one_byte_for_byte() {
    let (_scratch, shelf) = filed("fresh-none");
    let (session, asked) = cut(None, &shelf);
    assert_eq!(written(&session), blank());
    assert_eq!(asked, [None], "fitted to no document, as before");
}

#[test]
fn a_default_person_the_library_does_not_hold_leaves_the_blank_product() {
    let (scratch, _) = filed("fresh-gone");
    scratch.put("rota.toile-persona", "{\"toile_persona\": 1,");
    let mut shelf = Shelf::over(Some(scratch.library()));
    for stem in ["bea", "rota", "../etienne", "Etienne", ""] {
        let (session, asked) = cut(Some(stem), &shelf);
        assert_eq!(written(&session), blank(), "{stem:?}");
        assert_eq!(asked, [None], "{stem:?}");
    }

    std::fs::remove_file(scratch.file(STEM)).expect("Etienne's file is removed");
    shelf.refresh();
    let (session, _) = cut(Some(STEM), &shelf);
    assert_eq!(written(&session), blank(), "a person deleted since");
}

#[test]
fn a_library_that_cannot_be_read_leaves_the_blank_product() {
    let scratch = Scratch::new("fresh-unreadable");
    let not_a_folder = scratch.beside("personas");
    std::fs::write(&not_a_folder, "").expect("a file where the folder goes");
    let shelf = Shelf::over(Some(Library::at(not_a_folder)));
    assert!(shelf.listed().is_err(), "the folder does not read");
    assert_eq!(written(&cut(Some(STEM), &shelf).0), blank());
    assert_eq!(written(&cut(Some(STEM), &Shelf::over(None)).0), blank());
}
