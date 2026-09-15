use toile_engine::draft::{Persona, PersonaError};

use super::{Scratch, from_a_later_toile, person};
use crate::library::{LibraryError, Listed};

fn found<'a>(listed: &'a [Listed], stem: &str) -> &'a Result<Persona, LibraryError> {
    &listed
        .iter()
        .find(|entry| entry.stem == stem)
        .unwrap_or_else(|| panic!("{stem} is listed"))
        .persona
}

#[test]
fn every_file_that_is_not_a_person_is_listed_with_why() {
    let scratch = Scratch::new("broken");
    let library = scratch.library();
    library.create(&person("Ana")).expect("Ana is created");
    scratch.put("futura.toile-persona", &from_a_later_toile("Luz"));
    scratch.put("rota.toile-persona", "{ esto no es");
    scratch.put("vacia.toile-persona", "");
    let mayus = person("Mayus").to_canonical_json().expect("writable");
    scratch.put("Mayus.toile-persona", &mayus);
    std::fs::create_dir(scratch.folder().join("carpeta.toile-persona")).expect("a folder");
    scratch.put("notas.txt", "no es nadie");
    scratch.put("._ana.toile-persona", "\0\u{5}\u{16}\u{7}");
    scratch.put(".ana.1.0.tmp", "a save that never finished");

    let listed = library
        .list()
        .expect("a library with broken files still lists");
    let stems: Vec<&str> = listed.iter().map(|entry| entry.stem.as_str()).collect();
    assert_eq!(
        stems,
        ["Mayus", "ana", "carpeta", "futura", "rota", "vacia"]
    );
    assert!(found(&listed, "ana").is_ok());
    assert!(matches!(
        found(&listed, "Mayus"),
        Err(LibraryError::NotAStem(_))
    ));
    assert!(matches!(
        found(&listed, "carpeta"),
        Err(LibraryError::Io { .. })
    ));
    assert!(matches!(
        found(&listed, "futura"),
        Err(LibraryError::Unreadable {
            why: PersonaError::UnknownVersion { found: 2, .. },
            ..
        })
    ));
    assert!(matches!(
        found(&listed, "rota"),
        Err(LibraryError::Unreadable {
            why: PersonaError::NotJson(_),
            ..
        })
    ));
    assert!(matches!(
        found(&listed, "vacia"),
        Err(LibraryError::Unreadable {
            why: PersonaError::Truncated(_),
            ..
        })
    ));
    let said = found(&listed, "rota")
        .as_ref()
        .expect_err("broken")
        .to_string();
    assert!(said.contains("«rota.toile-persona»"), "{said}");
}

/// A save killed between writing its temporary file and putting it in place
/// leaves a whole copy of the person under a hidden name. Listing removes it
/// once the process that made it is gone, and never touches a running save's
/// file nor any name a save does not make.
#[cfg(unix)]
#[test]
fn a_copy_a_dead_save_left_behind_is_removed_and_nothing_else() {
    let scratch = Scratch::new("orphan");
    let library = scratch.library();
    library.create(&person("Ana")).expect("Ana is created");
    let mut child = std::process::Command::new("true")
        .spawn()
        .expect("a process to outlive");
    let dead = child.id();
    child.wait().expect("it ends");
    let orphan = format!(".ana.{dead}.0.tmp");
    let running = format!(".ana.{}.9.tmp", std::process::id());
    scratch.put(&orphan, "Ana, notas incluidas");
    scratch.put(&running, "a save still going");
    scratch.put(".ana.notas", "not a name a save makes");

    library.list().expect("the library lists");
    let names = scratch.names();
    assert!(!names.contains(&orphan), "{names:?}");
    assert!(names.contains(&running), "{names:?}");
    assert!(names.contains(&".ana.notas".to_owned()), "{names:?}");
    assert!(scratch.file("ana").exists());
}
