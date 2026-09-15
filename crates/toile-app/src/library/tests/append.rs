use toile_engine::draft::PersonaError;

use super::{Scratch, from_a_later_toile, person, session};
use crate::library::{Appended, LibraryError};

/// Ana in the library with one session, and the stem she took.
fn ana(scratch: &Scratch) -> String {
    scratch
        .library()
        .create(&person("Ana"))
        .expect("Ana is created")
}

#[test]
fn a_new_session_goes_after_every_earlier_one_and_keeps_the_notes() {
    let scratch = Scratch::new("append");
    let stem = ana(&scratch);
    let library = scratch.library();
    let before = library.load(&stem).expect("Ana loads");
    let saved = library
        .append(&stem, session("2026-09-01", 71.0))
        .expect("the session is saved");
    let Appended::Recorded(after) = saved else {
        panic!("a new session is recorded: {saved:?}");
    };
    assert_eq!(after.taken.len(), 2);
    assert_eq!(after.taken[0], before.taken[0]);
    assert_eq!(after.notes, before.notes);
    let waist = after.current().map(|current| current.values["cintura"]);
    assert_eq!(waist, Some(71.0));
    assert_eq!(library.load(&stem).expect("Ana loads"), after);
    let canonical = after.to_canonical_json().expect("writable");
    assert_eq!(scratch.bytes(&stem), canonical.as_bytes());
    assert_eq!(scratch.names(), ["ana.toile-persona"]);
}

#[test]
fn two_sessions_on_one_day_are_both_kept_in_the_order_saved() {
    let scratch = Scratch::new("same-day");
    let stem = ana(&scratch);
    let library = scratch.library();
    let morning = library.append(&stem, session("2026-09-15", 71.0));
    assert!(matches!(morning, Ok(Appended::Recorded(_))));
    let afternoon = library.append(&stem, session("2026-09-15", 70.5));
    assert!(matches!(afternoon, Ok(Appended::Recorded(_))));
    let taken = library.load(&stem).expect("Ana loads").taken;
    let waists: Vec<(&str, f64)> = taken
        .iter()
        .map(|each| (each.date.as_str(), each.values["cintura"]))
        .collect();
    let expected = [
        ("2026-03-02", 72.0),
        ("2026-09-15", 71.0),
        ("2026-09-15", 70.5),
    ];
    assert_eq!(waists, expected);
}

#[test]
fn saving_the_current_session_again_writes_nothing() {
    let scratch = Scratch::new("repeat");
    let stem = ana(&scratch);
    let library = scratch.library();
    library
        .append(&stem, session("2026-09-15", 71.0))
        .expect("saved");
    let written = scratch.bytes(&stem);
    let again = library
        .append(&stem, session("2026-09-15", 71.0))
        .expect("saved again");
    assert!(matches!(again, Appended::AlreadyCurrent(ref kept) if kept.taken.len() == 2));
    assert_eq!(scratch.bytes(&stem), written);
    let later = library
        .append(&stem, session("2026-09-16", 71.0))
        .expect("the same tape on another day");
    assert!(
        matches!(later, Appended::Recorded(ref kept) if kept.taken.len() == 3),
        "a session on another day is a session of its own"
    );
}

#[test]
fn a_session_dated_before_the_newest_is_refused_and_the_file_is_untouched() {
    let scratch = Scratch::new("backwards");
    let stem = ana(&scratch);
    let library = scratch.library();
    let written = scratch.bytes(&stem);
    let earlier = library.append(&stem, session("2025-12-31", 70.0));
    assert!(matches!(
        earlier,
        Err(LibraryError::Refused(PersonaError::OutOfOrder { .. }))
    ));
    let undated = library.append(&stem, session("mañana", 70.0));
    assert!(matches!(
        undated,
        Err(LibraryError::Refused(PersonaError::Invalid(_)))
    ));
    assert_eq!(scratch.bytes(&stem), written);
    assert_eq!(scratch.names(), ["ana.toile-persona"]);
}

#[test]
fn nobody_in_the_library_is_appended_to_and_nothing_is_written() {
    let scratch = Scratch::new("missing");
    let library = scratch.library();
    assert!(matches!(library.load("ana"), Err(LibraryError::Missing(ref stem)) if stem == "ana"));
    let appended = library.append("ana", session("2026-09-15", 70.0));
    assert!(matches!(appended, Err(LibraryError::Missing(_))));
    assert!(scratch.names().is_empty());
}

#[test]
fn a_file_this_build_cannot_read_is_never_rewritten() {
    let scratch = Scratch::new("later");
    let later = from_a_later_toile("Ana");
    scratch.put("ana.toile-persona", &later);
    let refused = scratch.library().append("ana", session("2026-09-15", 70.0));
    assert!(matches!(
        refused,
        Err(LibraryError::Unreadable {
            why: PersonaError::UnknownVersion { found: 2, .. },
            ..
        })
    ));
    assert_eq!(scratch.bytes("ana"), later.as_bytes());
}

/// A save that wrote into the file would show through a second name for it;
/// one that renames a new file into place leaves the old one whole under it.
#[test]
fn a_save_puts_a_whole_new_file_in_place_and_never_writes_into_the_old_one() {
    let scratch = Scratch::new("atomic");
    let stem = ana(&scratch);
    let before = scratch.bytes(&stem);
    let held = scratch.root.join("held");
    std::fs::hard_link(scratch.file(&stem), &held).expect("a second name for the file");
    scratch
        .library()
        .append(&stem, session("2026-09-15", 70.0))
        .expect("saved");
    assert_eq!(std::fs::read(&held).expect("the old file"), before);
    assert_ne!(scratch.bytes(&stem), before);
    assert_eq!(scratch.names(), ["ana.toile-persona"]);
}

#[cfg(unix)]
#[test]
fn a_save_the_disk_refuses_leaves_the_person_as_she_was() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = Scratch::new("read-only");
    let stem = ana(&scratch);
    let before = scratch.bytes(&stem);
    let folder = scratch.folder();
    let mode = |bits| std::fs::Permissions::from_mode(bits);
    std::fs::set_permissions(&folder, mode(0o555)).expect("the folder made read-only");
    // Root writes through a read-only folder, and then there is nothing to see.
    if std::fs::write(folder.join("probe"), "").is_ok() {
        std::fs::set_permissions(&folder, mode(0o755)).expect("the folder writable again");
        return;
    }
    let refused = scratch.library().append(&stem, session("2026-09-15", 70.0));
    std::fs::set_permissions(&folder, mode(0o755)).expect("the folder writable again");
    assert!(
        matches!(refused, Err(LibraryError::Io { .. })),
        "{refused:?}"
    );
    assert_eq!(scratch.bytes(&stem), before);
    assert_eq!(scratch.names(), ["ana.toile-persona"]);
}
