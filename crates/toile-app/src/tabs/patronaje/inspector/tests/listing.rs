use eframe::egui::{Key, Modifiers};
use toile_engine::draft::{Doc, MeasureSet};

use super::super::super::state::Field;
use super::super::cite::{measure_id, spliced};
use super::super::tape::groups;
use super::super::write::id_of;
use super::desk::{Desk, hip};

/// A product made from scratch carries a body with an empty tape, and the
/// names a pattern can read are the catalogue's, not that body's.
#[test]
fn an_empty_tape_lists_every_catalogue_name_without_a_value() {
    let set = MeasureSet::default();
    let listed = groups(&set);
    let headings: Vec<_> = listed.iter().map(|(heading, _)| *heading).collect();
    assert_eq!(headings, ["Contornos", "Largos y anchos", "Cuerpo"]);
    let mut names: Vec<_> = listed.iter().flat_map(|(_, names)| names.clone()).collect();
    assert!(names.iter().all(|(_, value)| value.is_none()), "{names:?}");
    names.sort_unstable_by(|a, b| a.0.cmp(b.0));
    let mut catalogue = MeasureSet::CATALOGUE.to_vec();
    catalogue.sort_unstable();
    assert_eq!(
        names.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        catalogue
    );

    let mut desk = Desk::tall(Doc::new(MeasureSet::default()));
    desk.frame(Vec::new());
    for name in MeasureSet::CATALOGUE {
        assert!(
            desk.ctx.read_response(measure_id(name)).is_some(),
            "{name} is listed"
        );
        let boxed = id_of(&Field::Measure(name.to_owned()));
        assert!(
            desk.ctx.read_response(boxed).is_none(),
            "{name} has no value to write"
        );
    }
}

/// A name the body carries outside the catalogue stays within reach.
#[test]
fn a_name_outside_the_catalogue_is_listed_after_it() {
    let set = MeasureSet::new("Etienne", [("cadera", 98.0), ("largo_manga", 60.0)]);
    let listed = groups(&set);
    assert_eq!(
        listed.last(),
        Some(&("Otras", vec![("largo_manga", Some(60.0))]))
    );
}

/// The list took over the measures section, and what that section did still
/// works: a measurement the body carries is written in place.
#[test]
fn a_carried_measurement_is_still_written_from_the_list() {
    let (mut desk, _) = hip();
    let cuello = id_of(&Field::Measure("cuello".to_owned()));
    assert!(
        desk.ctx.read_response(cuello).is_none(),
        "an uncarried name has no box"
    );

    desk.click(desk.centre(id_of(&Field::Measure("cadera".to_owned()))));
    desk.key(Key::A, Modifiers::COMMAND);
    desk.text("100");
    desk.key(Key::Enter, Modifiers::NONE);
    let body = desk
        .session
        .draft()
        .and_then(|d| d.doc().measures().cloned());
    let cadera = body.and_then(|set| set.get("cadera"));
    assert!(
        cadera.is_some_and(|cm| (cm - 100.0).abs() < 1e-9),
        "{cadera:?}"
    );
}

#[test]
fn a_name_is_spliced_by_character_not_by_byte() {
    assert_eq!(
        spliced("/4+1", 0..0, "cadera"),
        ("cadera/4+1".to_owned(), 6)
    );
    assert_eq!(
        spliced("cintura/4", 0..7, "cadera"),
        ("cadera/4".to_owned(), 6)
    );
    assert_eq!(spliced("·/2", 1..1, "tiro"), ("·tiro/2".to_owned(), 5));
    assert_eq!(spliced("1+", 9..9, "tiro"), ("1+tiro".to_owned(), 6));
}
