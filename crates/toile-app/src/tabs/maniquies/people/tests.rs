#![allow(
    clippy::float_cmp,
    reason = "a measurement reads back as the very number written into it"
)]

mod band;
mod listing;

use toile_engine::draft::{Doc, Persona, Snapshot};
use toile_engine::session::Session;

use super::super::stand::Stand;
use super::super::tests::{drag, product, resolved, written};
use super::{People, Plea, act};
use crate::band::Band;
use crate::library::shelf::Shelf;
use crate::library::tests::Scratch;
use crate::library::today::today;

/// The stem Ana is filed under.
const ANA: &str = "ana";

/// What Ana's notes say, which no product may repeat.
const NOTES: &str = "prefiere el tiro alto";

/// Ana as she was measured in March: the shipped block's whole tape, since its
/// variables read more of the body than the waist, with a waist of her own.
fn ana(session: &Session) -> Persona {
    let mut values = session
        .draft()
        .and_then(|draft| draft.doc().measures())
        .expect("a product is open")
        .values
        .clone();
    values.insert("cintura".to_owned(), 72.0);
    Persona {
        name: "Ana".to_owned(),
        notes: NOTES.to_owned(),
        taken: vec![Snapshot {
            date: "2026-03-02".to_owned(),
            values,
            phenotype: None,
        }],
    }
}

/// Everything one library flow touches: a scratch library, the app's read of
/// it, the product on the table and the tab over it.
struct Table {
    scratch: Scratch,
    shelf: Shelf,
    session: Session,
    stand: Stand,
    band: Band,
    people: People,
}

impl Table {
    /// The shipped block open, with Ana filed in a scratch library.
    fn with_ana(test: &str) -> Table {
        let scratch = Scratch::new(test);
        let (session, stand) = product();
        let stem = scratch
            .library()
            .create(&ana(&session))
            .expect("Ana is filed");
        assert_eq!(stem, ANA);
        let shelf = Shelf::over(Some(scratch.library()));
        Table {
            scratch,
            shelf,
            session,
            stand,
            band: Band::default(),
            people: People::default(),
        }
    }

    fn act(&mut self, plea: Plea) {
        act(
            plea,
            &mut self.session,
            &mut self.stand,
            &mut self.shelf,
            &mut self.band,
            &mut self.people,
        );
    }

    fn use_ana(&mut self) {
        self.act(Plea::Use(ANA.to_owned()));
    }

    /// The product closed and opened again from the bytes it would save, with
    /// the band the app works out for a product it opens.
    fn reopen(&mut self) {
        let doc = Doc::from_json(&written(&self.session)).expect("the product reads back");
        self.session = Session::from_doc(doc).expect("the product drapes");
        self.stand = Stand::default();
        self.band = Band::opened(&mut self.shelf, &self.session);
    }
}

/// A person used in the product is the body its pattern resolves with, linked
/// to her file; a second copy takes a name of its own; and one undo takes each
/// copy out whole.
#[test]
fn a_person_used_in_the_product_is_what_it_resolves_with_and_one_undo_takes_her_out() {
    let mut table = Table::with_ana("use");
    let first = table.session.draft().expect("a product").doc().resolve_with;

    table.use_ana();
    assert_eq!(resolved(&table.session, "cintura"), Some(72.0));
    let body = table.stand.body(&table.session);
    assert_eq!(body.name, "Ana");
    let origin = body.origin.as_ref().expect("the copy is linked");
    assert_eq!(origin.persona, ANA);
    assert_eq!(origin.taken, "2026-03-02");
    assert!(!written(&table.session).contains(NOTES), "notes stay home");
    assert_eq!(table.session.undo_label(), Some("usar persona"));

    table.use_ana();
    assert_eq!(table.stand.body(&table.session).name, "Ana 2");
    table.session.undo().expect("the second copy undoes");
    assert_eq!(table.stand.body(&table.session).name, "Ana");

    table.session.undo().expect("the first copy undoes");
    let doc = table.session.draft().expect("a product").doc();
    assert_eq!(doc.resolve_with, first);
    assert_eq!(doc.mannequin_named("Ana"), None);
    assert!(
        !table.session.can_undo(),
        "the copy and the choice were one entry"
    );
}

/// With no product open the person lands on the stand alone, and nothing is
/// written anywhere.
#[test]
fn with_no_product_a_person_is_used_on_the_stand_and_kept_nowhere() {
    let mut table = Table::with_ana("loose");
    table.session = Session::demo_bodice();
    table.use_ana();
    let body = table.stand.body(&table.session);
    assert_eq!(
        (body.name.as_str(), body.get("cintura")),
        ("Ana", Some(72.0))
    );
    assert_eq!(body.origin.as_ref().map(|o| o.persona.as_str()), Some(ANA));
    assert!(table.session.draft().is_none());
    assert_eq!(table.session.revision(), 0);
}

/// A body saved to the library adds a session dated today to its person, and
/// its link follows: no band on the spot, and none when the product reopens.
#[test]
fn saving_to_the_library_adds_a_dated_session_and_raises_no_band() {
    let mut table = Table::with_ana("save");
    table.use_ana();
    drag(&mut table.stand, &mut table.session, "cintura", &[70.0]);

    table.act(Plea::Save);
    let said = table.people.said();
    assert_eq!(said.map(|(_, bad)| bad), Some(false), "{said:?}");
    let filed = table.scratch.library().load(ANA).expect("Ana loads");
    assert_eq!(filed.taken.len(), 2, "the history grows");
    assert_eq!(filed.notes, NOTES);
    let current = filed.current().expect("Ana has sessions");
    assert_eq!(current.date, today());
    assert_eq!(current.values["cintura"], 70.0);
    let origin = table.stand.body(&table.session).origin.clone();
    let origin = origin.expect("the body stays linked");
    assert_eq!((origin.fnv, origin.taken), (current.fingerprint(), today()));
    assert_eq!(table.session.undo_label(), Some("guardar en biblioteca"));
    assert!(table.band.offers().is_empty());
    let listed = table.shelf.persona(ANA).map(|ana| ana.taken.len());
    assert_eq!(listed, Some(2), "the list was read again after the save");

    table.reopen();
    assert!(
        table.band.offers().is_empty(),
        "no band against her own save"
    );
}

/// A body with no link is filed as a new person. Undoing the save takes the
/// link back out of the product; the library keeps what was saved.
#[test]
fn a_body_with_no_link_is_filed_as_a_new_person() {
    let mut table = Table::with_ana("create");
    let before = written(&table.session);

    table.act(Plea::Save);
    let filed = table.scratch.library().load("etienne").expect("filed");
    let origin = table.stand.body(&table.session).origin.clone();
    let origin = origin.expect("the body is linked now");
    assert_eq!(origin.persona, "etienne");
    assert_eq!(
        origin.fnv,
        filed.current().expect("a session").fingerprint()
    );
    assert!(table.shelf.persona("etienne").is_some());

    table.session.undo().expect("the link undoes");
    assert_eq!(written(&table.session), before);
    assert!(table.scratch.library().load("etienne").is_ok());
}

/// A save the disk refuses is said, in the panel's words, and neither the
/// product nor the person's file moves.
#[cfg(unix)]
#[test]
fn a_save_the_disk_refuses_is_said_and_leaves_the_product_alone() {
    use std::os::unix::fs::PermissionsExt;

    let mut table = Table::with_ana("read-only");
    table.use_ana();
    drag(&mut table.stand, &mut table.session, "cintura", &[70.0]);
    let revision = table.session.revision();
    let (product, file) = (written(&table.session), table.scratch.bytes(ANA));
    let folder = table.scratch.folder();
    let mode = |bits| std::fs::Permissions::from_mode(bits);
    std::fs::set_permissions(&folder, mode(0o555)).expect("the folder made read-only");
    // Root writes through a read-only folder, and then there is nothing to see.
    if std::fs::write(folder.join("probe"), "").is_ok() {
        std::fs::set_permissions(&folder, mode(0o755)).expect("writable again");
        return;
    }
    table.act(Plea::Save);
    std::fs::set_permissions(&folder, mode(0o755)).expect("writable again");

    let (text, bad) = table.people.said().expect("the failure is said");
    assert!(bad, "{text}");
    assert!(
        text.starts_with("no se pudo guardar en la biblioteca"),
        "{text}"
    );
    assert_eq!(table.session.revision(), revision);
    assert_eq!(written(&table.session), product);
    assert_eq!(table.scratch.bytes(ANA), file);
}
