#![allow(
    clippy::float_cmp,
    reason = "a measurement reads back as the very number written into it"
)]

mod append;
mod listing;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use toile_engine::draft::{BodyShape, Persona, PersonaError, Snapshot};

use super::today::today;
use super::{Library, LibraryError, PERSONA_EXT};

/// A fresh folder under the system's temporary directory, removed when the
/// test ends.
///
/// The library under test is only ever given a folder inside it, so whatever
/// a test does, it cannot reach the person's real library.
pub(crate) struct Scratch {
    root: PathBuf,
}

impl Scratch {
    pub(crate) fn new(test: &str) -> Scratch {
        static MADE: AtomicU32 = AtomicU32::new(0);
        let n = MADE.fetch_add(1, Ordering::Relaxed);
        let name = format!("toile-library-{}-{n}-{test}", std::process::id());
        let root = std::env::temp_dir().join(name);
        std::fs::create_dir(&root).expect("a scratch folder nobody else has");
        Scratch { root }
    }

    /// The library, in a folder of the scratch one that is not made yet.
    pub(crate) fn library(&self) -> Library {
        Library::at(self.folder())
    }

    pub(crate) fn folder(&self) -> PathBuf {
        self.root.join("personas")
    }

    /// A file in the scratch folder beside the library, such as the
    /// preferences the app keeps next to it.
    pub(crate) fn beside(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    pub(crate) fn file(&self, stem: &str) -> PathBuf {
        self.folder().join(format!("{stem}.{PERSONA_EXT}"))
    }

    /// A person's file, byte for byte.
    pub(crate) fn bytes(&self, stem: &str) -> Vec<u8> {
        std::fs::read(self.file(stem)).expect("the file is there")
    }

    /// Writes a file into the library by hand, as another tool might.
    pub(crate) fn put(&self, name: &str, text: &str) {
        std::fs::create_dir_all(self.folder()).expect("the library folder");
        std::fs::write(self.folder().join(name), text).expect("a hand-made file");
    }

    /// Every name in the library folder, hidden ones too, in order.
    fn names(&self) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(self.folder()) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .map(|entry| {
                let entry = entry.expect("a folder entry");
                entry.file_name().to_string_lossy().into_owned()
            })
            .collect();
        names.sort();
        names
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if self.root.starts_with(std::env::temp_dir()) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
}

/// One session with the tape.
fn session(date: &str, waist: f64) -> Snapshot {
    Snapshot {
        date: date.to_owned(),
        values: BTreeMap::from([("cadera".to_owned(), 96.0), ("cintura".to_owned(), waist)]),
        phenotype: None,
    }
}

/// A person with one session, and notes that must stay in her file.
fn person(name: &str) -> Persona {
    Persona {
        name: name.to_owned(),
        notes: "prefiere el tiro alto".to_owned(),
        taken: vec![session("2026-03-02", 72.0)],
    }
}

/// A person's file as a later Toile, with a format this build does not read,
/// would write it.
fn from_a_later_toile(name: &str) -> String {
    person(name)
        .to_canonical_json()
        .expect("writable")
        .replacen("\"toile_persona\": 1", "\"toile_persona\": 2", 1)
}

#[test]
fn an_empty_library_lists_nobody_and_makes_no_folder() {
    let scratch = Scratch::new("empty");
    let listed = scratch.library().list().expect("no folder is no one");
    assert!(listed.is_empty());
    assert!(!scratch.folder().exists());
}

#[test]
fn a_person_created_is_one_canonical_file_that_reads_back_the_same() {
    let scratch = Scratch::new("create");
    let library = scratch.library();
    let ana = person("Ana");
    assert_eq!(library.create(&ana).expect("Ana is created"), "ana");
    assert_eq!(scratch.names(), ["ana.toile-persona"]);
    let canonical = ana.to_canonical_json().expect("Ana is writable");
    assert_eq!(scratch.bytes("ana"), canonical.as_bytes());
    assert_eq!(library.load("ana").expect("Ana loads"), ana);
    let listed = library.list().expect("the library lists");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].stem, "ana");
    assert_eq!(listed[0].persona.as_ref().ok(), Some(&ana));
}

#[test]
fn a_shaped_body_survives_the_library() {
    let scratch = Scratch::new("shape");
    let mut ana = person("Ana");
    ana.taken[0].phenotype = Some(BodyShape {
        sex: 1.0,
        age_years: 34.0,
        build: 0.62,
        muscle: 0.4,
        proportions: 0.25,
    });
    let stem = scratch.library().create(&ana).expect("Ana is created");
    assert_eq!(scratch.library().load(&stem).expect("Ana loads"), ana);
}

#[test]
fn a_taken_stem_gains_a_number_and_no_file_is_written_over() {
    let scratch = Scratch::new("collide");
    let library = scratch.library();
    assert_eq!(library.create(&person("Ana")).expect("the first"), "ana");
    let first = scratch.bytes("ana");
    scratch.put("ana-2.toile-persona", "hecho a mano");
    let mut other = person("ANA");
    other.notes = "otra Ana".to_owned();
    assert_eq!(library.create(&other).expect("the second"), "ana-3");
    assert_eq!(library.create(&person("Ana")).expect("the third"), "ana-4");
    assert_eq!(scratch.bytes("ana"), first);
    assert_eq!(scratch.bytes("ana-2"), b"hecho a mano");
    assert_eq!(library.load("ana-3").expect("the other").notes, "otra Ana");
    let names = [
        "ana-2.toile-persona",
        "ana-3.toile-persona",
        "ana-4.toile-persona",
        "ana.toile-persona",
    ];
    assert_eq!(scratch.names(), names);
}

#[test]
fn an_accented_name_is_filed_under_its_folded_stem() {
    let scratch = Scratch::new("accents");
    let library = scratch.library();
    assert_eq!(
        library.create(&person("Begoña")).expect("created"),
        "begona"
    );
    let stem = library.create(&person("José Ángel")).expect("created");
    assert_eq!(stem, "jose-angel");
    assert_eq!(library.load(&stem).expect("loads").name, "José Ángel");
}

#[test]
fn a_person_the_format_would_refuse_never_reaches_the_disk() {
    let scratch = Scratch::new("refused");
    let library = scratch.library();
    let nobody = Persona {
        taken: Vec::new(),
        ..person("Ana")
    };
    assert!(matches!(
        library.create(&nobody),
        Err(LibraryError::Refused(PersonaError::NothingTaken))
    ));
    let mut undated = person("Ana");
    undated.taken[0].date = "15/09/2026".to_owned();
    assert!(matches!(
        library.create(&undated),
        Err(LibraryError::Refused(PersonaError::Invalid(_)))
    ));
    assert!(!scratch.folder().exists());
}

#[test]
fn a_stem_that_could_leave_the_library_or_alias_a_file_is_refused() {
    let scratch = Scratch::new("climb");
    let library = scratch.library();
    library.create(&person("Ana")).expect("Ana is created");
    let beside = scratch.root.join("fuera.toile-persona");
    let outside = person("Fuera").to_canonical_json().expect("writable");
    std::fs::write(&beside, &outside).expect("a file beside the library");
    for stem in ["../fuera", "ana/../ana", "", "Ana", "ana.toile-persona"] {
        assert!(
            matches!(library.load(stem), Err(LibraryError::NotAStem(_))),
            "{stem}"
        );
        let appended = library.append(stem, session("2026-09-15", 70.0));
        assert!(matches!(appended, Err(LibraryError::NotAStem(_))), "{stem}");
    }
    assert_eq!(
        std::fs::read(&beside).expect("still there"),
        outside.as_bytes()
    );
    assert_eq!(library.load("ana").expect("Ana loads").taken.len(), 1);
}

#[test]
fn today_is_a_day_a_session_can_be_saved_under() {
    let scratch = Scratch::new("today");
    let day = today();
    let ana = Persona {
        taken: vec![session(&day, 72.0)],
        ..person("Ana")
    };
    let stem = scratch
        .library()
        .create(&ana)
        .expect("dated today, written");
    let loaded = scratch.library().load(&stem).expect("and read back");
    assert_eq!(loaded.taken[0].date, day);
}
