#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_doc::{
    Axis, Binding, Command, Doc, FORMAT_VERSION, FormatError, Grain, MeasureSet, Point, PointKey,
    block,
};

/// A pattern as a person would type it: the required fields and nothing else.
const BY_HAND: &str = r#"{
  "toile": 1,
  "doc": {
    "pieces": { "issued": 1, "entries": [
      { "id": "0.0", "name": "Delantero", "winding": "cw", "contour": [
        { "point": "0.0", "segment": { "kind": "line" }, "samples": 1 },
        { "point": "1.0", "segment": { "kind": "line" }, "samples": 1 },
        { "point": "2.0", "segment": { "kind": "line" }, "samples": 1 }
      ] }
    ] },
    "points": { "issued": 3, "entries": [
      { "id": "0.0", "x": 0, "y": 0 },
      { "id": "1.0", "x": "cintura / 4", "y": 0 },
      { "id": "2.0", "x": 0, "y": 10 }
    ] },
    "seams": { "issued": 0, "entries": [] },
    "notches": { "issued": 0, "entries": [] },
    "darts": { "issued": 0, "entries": [] },
    "symmetries": { "issued": 0, "entries": [] },
    "pins": { "issued": 0, "entries": [] },
    "variables": { "issued": 0, "entries": [] },
    "mannequins": { "issued": 1, "entries": [
      { "id": "0.0", "name": "Etienne", "values": { "cintura": 84 } }
    ] },
    "resolve_with": "0.0"
  }
}"#;

fn waist(doc: &Doc) -> PointKey {
    let front = doc.piece_named(block::FRONT).expect("the block draws one");
    doc.shows_label(front, "cintura_lat")
        .expect("the block names it")
}

fn written(doc: &Doc) -> String {
    doc.to_canonical_json()
}

fn reread(doc: &Doc) -> Doc {
    Doc::from_json(&written(doc)).expect("what the writer wrote, the reader reads")
}

#[test]
fn round_trip_is_byte_identical() {
    let mut doc = block::trouser_front();
    let spare = doc.points.insert(Point::at(1.0, 2.0));
    doc.points.insert(Point::at(3.0, 4.0));
    doc.points.remove(spare).expect("the key is live");
    let once = written(&doc);
    assert_eq!(written(&reread(&doc)), once);
    assert_eq!(reread(&doc), doc);
}

#[test]
fn the_gaps_and_the_generations_survive_the_round_trip() {
    let mut doc = block::trouser_front();
    let spare = doc.points.insert(Point::at(1.0, 2.0));
    doc.points.remove(spare).expect("the key is live");
    let read = reread(&doc);
    assert_eq!(read.points.issued(), doc.points.issued());
    assert_eq!(read.points.get(spare), None);
    assert_eq!(read.points.len(), doc.points.len());
}

#[test]
fn moving_one_point_changes_one_line() {
    let mut doc = block::trouser_front();
    let before = written(&doc);
    Command::SetBinding {
        point: waist(&doc),
        axis: Axis::X,
        to: Binding::literal(23.5),
    }
    .apply(&mut doc)
    .expect("the point is live");
    let after = written(&doc);
    let differ: Vec<(&str, &str)> = before
        .lines()
        .zip(after.lines())
        .filter(|(here, there)| here != there)
        .collect();
    assert_eq!(before.lines().count(), after.lines().count());
    assert_eq!(
        differ,
        [(
            "          \"x\": \"cintura / 4 + 1\",",
            "          \"x\": 23.5,"
        )]
    );
}

#[test]
fn a_formula_serialises_as_its_source() {
    let doc = block::trouser_front();
    assert!(written(&doc).contains("\"x\": \"cadera / 4 + holgura_cadera\","));
    assert!(
        written(&doc).contains("\"value\": \"(cadera / 4 + holgura_cadera - extension_tiro) / 2\"")
    );
    let read = reread(&doc);
    let point = read.points.get(waist(&read)).expect("the key is live");
    assert_eq!(point.x.source(), "cintura / 4 + 1");
}

#[test]
fn negative_zero_is_written_as_zero() {
    let mut doc = block::trouser_front();
    Command::MovePoint {
        point: waist(&doc),
        to: [Binding::literal(-0.0), Binding::literal(-0.5)],
    }
    .apply(&mut doc)
    .expect("the point is live");
    let file = written(&doc);
    assert!(file.contains("\"x\": 0,"), "{file}");
    assert!(!file.contains("-0,"), "{file}");
    assert!(file.contains("\"y\": -0.5,"), "{file}");
}

#[test]
fn a_number_is_never_written_as_a_power_of_ten() {
    let mut doc = block::trouser_front();
    Command::SetGrain {
        piece: doc.piece_named(block::FRONT).expect("the block draws one"),
        to: Grain::Angle(0.000_001),
    }
    .apply(&mut doc)
    .expect("the piece is live");
    assert!(written(&doc).contains("\"radians\": 0.000001"));
    assert_eq!(reread(&doc), doc);
}

/// A number that needs all seventeen digits is where a reader that hurries
/// lands one bit away from the number the writer wrote.
#[test]
fn a_number_reads_back_as_the_very_bits_it_was_written_from() {
    let mut doc = block::trouser_front();
    let awkward = 5.03 - 20.0;
    Command::MovePoint {
        point: waist(&doc),
        to: [Binding::literal(awkward), Binding::literal(0.1 + 0.2)],
    }
    .apply(&mut doc)
    .expect("the point is live");
    let file = written(&doc);
    assert!(file.contains("\"x\": -14.969999999999999,"), "{file}");
    assert!(file.contains("\"y\": 0.30000000000000004,"), "{file}");
    assert_eq!(written(&reread(&doc)), file);
}

#[test]
fn an_unknown_version_is_a_legible_error() {
    let file = written(&block::trouser_front()).replace("\"toile\": 1", "\"toile\": 2");
    let error = Doc::from_json(&file).expect_err("this build reads one version");
    assert_eq!(
        error,
        FormatError::UnknownVersion {
            found: 2,
            supported: FORMAT_VERSION,
        }
    );
    assert!(error.to_string().contains("version 2"));
}

#[test]
fn an_absent_optional_field_loads_with_its_default() {
    let doc = Doc::from_json(BY_HAND).expect("the required fields are all there");
    let front = doc.piece_named("Delantero").expect("the file draws one");
    assert_eq!(
        doc.pieces.get(front).expect("the key is live").grain,
        Grain::VERTICAL
    );
    let point = doc
        .points
        .get(PointKey::new(0, 0))
        .expect("the key is live");
    assert_eq!(point.label, None);
    assert!(!point.label_visible);
}

#[test]
fn a_file_a_person_typed_is_written_back_canonically() {
    let doc = Doc::from_json(BY_HAND).expect("the required fields are all there");
    let canonical = written(&doc);
    assert!(canonical.contains("\"label\": null,"));
    assert!(canonical.contains("\"label_visible\": false"));
    assert!(canonical.ends_with("}\n"));
    assert_eq!(written(&reread(&doc)), canonical);
}

#[test]
fn a_truncated_file_says_it_was_cut_off() {
    let file = written(&block::trouser_front());
    let cut = &file[..file.len() / 2];
    assert!(matches!(
        Doc::from_json(cut),
        Err(FormatError::Truncated(_))
    ));
}

#[test]
fn a_file_that_is_not_json_is_refused_by_its_first_fault() {
    assert!(matches!(
        Doc::from_json("this is a pattern, honestly"),
        Err(FormatError::NotJson(_))
    ));
    assert_eq!(Doc::from_json("[]"), Err(FormatError::NoHeader));
}

#[test]
fn a_field_of_the_wrong_shape_names_itself() {
    let file = written(&block::trouser_front()).replace("\"winding\": \"cw\"", "\"winding\": 3");
    let Err(FormatError::Malformed(told)) = Doc::from_json(&file) else {
        panic!("a winding of 3 is not a winding");
    };
    assert!(told.contains("invalid type: integer `3`"), "{told}");
    assert!(told.contains("at line "), "{told}");
}

#[test]
fn a_key_that_leads_nowhere_is_an_error_and_not_a_drawing() {
    let file =
        written(&block::trouser_front()).replace("\"point\": \"4.0\"", "\"point\": \"40.0\"");
    let error = Doc::from_json(&file).expect_err("the file carries nine points");
    assert!(
        error.to_string().contains("`Point` has no entry 40.0"),
        "{error}"
    );
}

#[test]
fn a_file_nested_deep_enough_to_bury_the_stack_is_an_error_and_not_a_crash() {
    let deep = format!("{}{}", "[".repeat(20_000), "]".repeat(20_000));
    assert!(matches!(
        Doc::from_json(&deep),
        Err(FormatError::Malformed(_) | FormatError::NotJson(_))
    ));
}

#[test]
fn a_count_too_big_for_the_field_that_holds_it_is_refused() {
    let file = written(&block::trouser_front()).replace("\"samples\": 1", "\"samples\": 99999");
    assert!(matches!(
        Doc::from_json(&file),
        Err(FormatError::Malformed(_))
    ));
}

/// The count sizes the flattening every resolve walks, pairwise and twice
/// over, so a file left to name its own is a file naming how long opening it
/// takes. The refusal is at the door and it costs nothing: the polyline the
/// count asks for is never built.
#[test]
fn a_flattening_no_tract_can_carry_is_refused_at_the_door() {
    let block = written(&block::trouser_front());
    for count in ["0", "1", "97", "65535"] {
        let file = block.replace("\"samples\": 24", &format!("\"samples\": {count}"));
        let error = Doc::from_json(&file).expect_err("the count is out of range");
        assert!(
            matches!(error, FormatError::Sampling(_)),
            "{count}: {error}"
        );
        assert!(error.to_string().contains(count), "{error}");
    }
    // A straight tract gives its own node and stops, so one describes it.
    assert!(Doc::from_json(&block).is_ok());
}

#[test]
fn a_value_json_cannot_spell_is_refused_on_the_way_back_in() {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", f64::NAN)]));
    doc.points.insert(Point::at(f64::INFINITY, 0.0));
    let file = written(&doc);
    assert!(file.contains("\"cintura\": null"), "{file}");
    assert!(matches!(
        Doc::from_json(&file),
        Err(FormatError::Malformed(_))
    ));
}
