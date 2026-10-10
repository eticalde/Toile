use std::path::{Path, PathBuf};

use toile_doc::{FORMAT_VERSION_CUT, Origin, Persona, Snapshot};
use toile_engine::draft::Doc;

use super::super::args::{Asked, Wanted, parse};
use super::super::migrate;
use super::{PATTERN, folder};

fn asked(input: &Path, output: PathBuf, library: &Path) -> Asked {
    filing(input, output, library, false)
}

fn filing(input: &Path, output: PathBuf, library: &Path, existing: bool) -> Asked {
    Asked {
        input: input.to_owned(),
        output,
        persona: Some(Wanted {
            name: "Etienne".to_owned(),
            taken: "2026-09-05".to_owned(),
            library: library.to_owned(),
            existing,
        }),
    }
}

#[test]
fn a_person_is_filed_as_the_app_files_her_and_the_product_carries_her_link() {
    let (root, input) = folder("persona");
    let library = root.join("personas");
    let output = input.with_extension("toile");
    let lines = migrate(&asked(&input, output.clone(), &library)).expect("migrates");
    assert!(
        lines.iter().any(|l| l.starts_with("persona «Etienne»")),
        "{lines:?}"
    );

    let names: Vec<_> = std::fs::read_dir(&library)
        .expect("the library folder was made")
        .map(|entry| entry.expect("an entry").file_name())
        .collect();
    assert_eq!(
        names,
        ["etienne.toile-persona"],
        "one file, no temporary left"
    );
    let filed = std::fs::read_to_string(library.join("etienne.toile-persona")).expect("filed");
    let persona = Persona::from_json(&filed).expect("the app's reader reads her");
    assert_eq!(persona.to_canonical_json().as_deref(), Ok(filed.as_str()));
    assert_eq!(persona.name, "Etienne");
    assert_eq!(persona.taken.len(), 1);
    assert_eq!(persona.taken[0].date, "2026-09-05");
    assert!(
        persona.notes.contains("measurements-eti.smis"),
        "{}",
        persona.notes
    );
    assert_eq!(Origin::stem_of(&persona.name), "etienne");

    let written = std::fs::read_to_string(&output).expect("the product is written");
    let doc = Doc::from_json(&written).expect("a Toile file");
    // The pieces carry what their author wrote about cutting them, which is
    // the newest thing in the file and so the version it asks for.
    assert_eq!(doc.format_version(), FORMAT_VERSION_CUT);
    assert_eq!(doc.to_canonical_json(), written);
    let body = doc.measures().expect("one body");
    assert_eq!(body.name, "Etienne");
    assert_eq!(Some(&body.values), persona.current().map(|s| &s.values));
    let origin = body.origin.as_ref().expect("the body is linked");
    assert_eq!(origin.persona, "etienne");
    assert_eq!(origin.taken, "2026-09-05");
    assert_eq!(
        Some(origin.fnv.clone()),
        persona.current().map(Snapshot::fingerprint)
    );
    assert!(!persona.differs_from(origin));

    let report = input.with_file_name("Baggy Jeans [Muller] - importado a Toile.md");
    let report = std::fs::read_to_string(report).expect("the report is written");
    assert!(report.contains("copia de la persona «Etienne»"), "{report}");
    for text in [&filed, &written, &report] {
        assert!(!text.contains("SENTINEL"), "the personal block leaked");
    }
    std::fs::remove_dir_all(root).expect("the scratch folder goes");
}

#[test]
fn a_person_already_in_the_library_is_never_written_over_and_nothing_else_is_written() {
    let (root, input) = folder("persona-twice");
    let library = root.join("personas");
    migrate(&asked(&input, input.with_extension("toile"), &library)).expect("the first run");
    let filed = library.join("etienne.toile-persona");
    let before = std::fs::read(&filed).expect("filed");

    let elsewhere = input.with_file_name("otra.toile");
    let refused = migrate(&asked(&input, elsewhere.clone(), &library)).expect_err("refused");
    assert!(refused.contains("ya hay una persona"), "{refused}");
    assert!(refused.contains("etienne.toile-persona"), "{refused}");
    assert!(refused.contains("--vincular"), "{refused}");
    assert_eq!(std::fs::read(&filed).expect("still there"), before);
    assert!(!elsewhere.exists());
    assert!(!input.with_file_name("otra.svg").exists());
    std::fs::remove_dir_all(root).expect("the scratch folder goes");
}

/// Asked to link, the run reads the library's own person, links the product to
/// the measuring she carries, and writes not one byte of the library.
#[test]
fn a_second_run_links_the_product_to_the_person_already_filed_and_leaves_her_alone() {
    let (root, input) = folder("persona-link");
    let library = root.join("personas");
    let first = input.with_extension("toile");
    migrate(&asked(&input, first.clone(), &library)).expect("the first run files her");
    let filed = library.join("etienne.toile-persona");
    let before = std::fs::read(&filed).expect("filed");
    let product = std::fs::read_to_string(&first).expect("written");

    let again = input.with_file_name("otra vez.toile");
    let lines = migrate(&filing(&input, again.clone(), &library, true)).expect("links");
    assert!(
        lines
            .iter()
            .any(|line| line.contains("ya en la biblioteca, que no se toca")),
        "{lines:?}"
    );
    assert!(lines.iter().all(|line| !line.contains("etienne.toile")));
    assert_eq!(std::fs::read(&filed).expect("still there"), before);
    // The same person, so the same product: linking twice is the same link.
    assert_eq!(std::fs::read_to_string(&again).expect("written"), product);
    let report = std::fs::read_to_string(input.with_file_name("otra vez - importado a Toile.md"))
        .expect("the report is written");
    assert!(report.contains("la biblioteca no se toca"), "{report}");
    std::fs::remove_dir_all(root).expect("the scratch folder goes");
}

/// The two ways of naming a person are not interchangeable: linking needs her
/// there, filing needs her absent, and a day that is not her newest measuring
/// is refused before anything is written.
#[test]
fn linking_needs_the_person_and_the_day_she_was_last_measured() {
    let (root, input) = folder("persona-link-refused");
    let library = root.join("personas");
    let output = input.with_extension("toile");

    let empty = migrate(&filing(&input, output.clone(), &library, true)).expect_err("refused");
    assert!(empty.contains("no hay ninguna persona"), "{empty}");
    assert!(empty.contains("--vincular"), "{empty}");
    assert!(!output.exists());

    migrate(&asked(&input, output.clone(), &library)).expect("files her");
    let mut wrong = filing(&input, input.with_file_name("otra.toile"), &library, true);
    if let Some(person) = wrong.persona.as_mut() {
        "2026-09-06".clone_into(&mut person.taken);
    }
    let refused = migrate(&wrong).expect_err("refused");
    assert!(refused.contains("2026-09-05"), "{refused}");
    assert!(!input.with_file_name("otra.toile").exists());
    std::fs::remove_dir_all(root).expect("the scratch folder goes");
}

#[test]
fn a_day_not_written_year_month_day_is_refused_before_any_write() {
    let (root, input) = folder("persona-day");
    let library = root.join("personas");
    let mut wanted = asked(&input, input.with_extension("toile"), &library);
    if let Some(person) = wanted.persona.as_mut() {
        "06/09/2026".clone_into(&mut person.taken);
    }
    let refused = migrate(&wanted).expect_err("refused");
    assert!(refused.contains("AAAA-MM-DD"), "{refused}");
    assert!(!library.exists());
    assert!(!input.with_extension("toile").exists());
    std::fs::remove_dir_all(root).expect("the scratch folder goes");
}

#[test]
fn a_pattern_without_pieces_is_refused_in_spanish_and_nothing_is_written() {
    let (root, input) = folder("no-pieces");
    let bare: String = PATTERN
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("<pieces>") {
                "        <pieces/>"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&input, bare).expect("written");
    let output = input.with_extension("toile");
    let asked = Asked {
        input: input.clone(),
        output: output.clone(),
        persona: None,
    };
    let refused = migrate(&asked).expect_err("refused");
    assert!(refused.contains("no tiene piezas"), "{refused}");
    assert!(!output.exists());
    assert!(!input.with_extension("svg").exists());
    std::fs::remove_dir_all(root).expect("the scratch folder goes");
}

#[test]
fn the_person_flags_go_together_in_any_order_and_without_them_nothing_changes() {
    let args = |list: &[&str]| parse(&list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>());
    let plain = args(&["a.sm2d"]).expect("a pattern alone");
    assert_eq!(plain.output, PathBuf::from("a.toile"));
    assert_eq!(plain.persona, None);
    let full = args(&[
        "--tomada",
        "2026-09-05",
        "a.sm2d",
        "--biblioteca",
        "lib",
        "b.toile",
        "--persona",
        "Ana María",
    ])
    .expect("the flags anywhere");
    assert_eq!(full.output, PathBuf::from("b.toile"));
    let wanted = full.persona.expect("a person");
    assert_eq!(
        (wanted.name.as_str(), wanted.taken.as_str()),
        ("Ana María", "2026-09-05")
    );
    assert_eq!(wanted.library, PathBuf::from("lib"));
    assert!(!wanted.existing, "filing her is what no flag asks for");
    let linking = args(&[
        "a.sm2d",
        "--persona",
        "Ana",
        "--tomada",
        "2026-09-05",
        "--biblioteca",
        "lib",
        "--vincular",
    ])
    .expect("the four flags together");
    assert!(linking.persona.expect("a person").existing);
    for wrong in [
        &["a.sm2d", "--persona", "Ana"][..],
        &["a.sm2d", "--persona", "--tomada", "2026-09-05"],
        &["a.sm2d", "--otra"],
        // Linking says which person to link to, so it cannot stand alone.
        &["a.sm2d", "--vincular"],
        &[],
    ] {
        assert!(args(wrong).is_err(), "{wrong:?}");
    }
}
