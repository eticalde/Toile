use toile_doc::{Binding, MeasureSet};
use toile_seamly::{HelperKind, measurement_pairs};

use super::{BODY, NAME, body, product};

#[test]
fn the_one_body_is_named_as_asked_and_carries_every_mapped_measurement() {
    let product = product();
    assert_eq!(product.doc.mannequins.len(), 1);
    let set = product
        .doc
        .measures()
        .expect("the product resolves against its body");
    assert_eq!(set.name, NAME);
    let owner = body();
    let mapped: Vec<_> = measurement_pairs()
        .filter(|(seamly, _)| owner.get(seamly).is_some())
        .collect();
    assert_eq!(set.values.len(), mapped.len());
    for (seamly, toile) in mapped {
        assert_eq!(set.get(toile), owner.get(seamly), "{toile}");
        assert!(MeasureSet::is_catalogued(toile));
    }
    assert!(set.uncatalogued().is_empty());
}

/// The jeans that were cut were drafted on this body's inseam, not on the one
/// the shipped block's body carries.
#[test]
fn the_body_measures_the_inseam_and_the_hip_the_jeans_were_cut_on() {
    let product = product();
    let set = product.doc.measures().expect("a body");
    assert_eq!(set.get("entrepierna"), Some(80.5));
    assert_eq!(set.get("cadera"), Some(98.0));
}

#[test]
fn every_measurement_is_either_mapped_or_reported_as_left_out() {
    let report = product().report;
    assert!(report.carried.is_empty());
    let named = report.mapped.len() + report.left_out.len();
    assert_eq!(named, body().entries().len());
    assert!(report.left_out.iter().all(|m| m.toile.is_none()));
}

/// Nothing the measurement file's `<personal>` block says reaches the
/// product, not even through the body's name.
///
/// The fixture's block holds sentinels in place of the owner's details, so
/// a leak shows as a word no product would otherwise contain.
#[test]
fn the_personal_block_never_reaches_the_product() {
    let open = BODY.find("<personal>").expect("the file has one");
    let close = BODY.find("</personal>").expect("and closes it");
    let said: Vec<&str> = BODY[open..close]
        .split(['<', '>'])
        .step_by(2)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .collect();
    assert_eq!(
        said,
        [
            "SENTINEL-GIVEN-NAME",
            "SENTINEL-BIRTH-DATE",
            "SENTINEL-GENDER"
        ]
    );
    let product = product();
    let written = format!("{}{:?}", product.doc.to_canonical_json(), product.report);
    assert!(
        !written.contains("SENTINEL"),
        "the personal block reached the product"
    );
}

#[test]
fn every_pattern_variable_keeps_its_name_its_formula_and_its_note() {
    let product = product();
    let report = &product.report;
    assert_eq!(report.variables.len(), 18);
    for note in &report.variables {
        let key = product.doc.variable_named(&note.toile).expect("carried");
        let held = product.doc.variables.get(key).expect("live");
        assert_eq!(Binding::parse(&note.formula).as_ref(), Ok(&held.value));
        assert_eq!(
            note.toile,
            note.seamly.trim_start_matches('#').to_ascii_lowercase()
        );
        assert!(!note.description.is_empty(), "{}", note.seamly);
    }
    let note = |seamly: &str| {
        report
            .variables
            .iter()
            .find(|v| v.seamly == seamly)
            .expect("carried")
    };
    assert_eq!(note("#PhL").toile, "phl");
    assert_eq!(note("#CordD").toile, "cordd");
    assert_eq!(note("#Ftcw").formula, "(cadera/2/10)+1");
    assert_eq!(
        note("#Knh").formula,
        "(entrepierna/2) + (entrepierna/10) - 2"
    );
}

#[test]
fn the_helper_variables_are_the_back_yoke_and_the_lengths_its_formulas_cite() {
    let report = product().report;
    let drawn: Vec<&str> = report
        .helpers
        .iter()
        .filter_map(|h| match &h.stands_for {
            HelperKind::Drawn(name) => Some(name.as_str()),
            HelperKind::Coordinate(_) => None,
        })
        .collect();
    assert_eq!(
        drawn,
        [
            "Line_b_crotch_b_knee_start",
            "Spl_crotch_waist_margin_up",
            "Line_bk_waist_side_b_knee_mid",
            "Line_b_knee_end_B1"
        ]
    );
    let mut points: Vec<&str> = report
        .helpers
        .iter()
        .filter_map(|h| match &h.stands_for {
            HelperKind::Coordinate(point) => Some(point.as_str()),
            HelperKind::Drawn(_) => None,
        })
        .collect();
    points.dedup();
    // The first seven are corners of the back and the points its corners are
    // built on. The rest are places of internal lines: the fly topstitch, and
    // the back pocket's flap, slit and bag guide, which hang off the same yoke
    // heading and so need the same square root written down once.
    assert_eq!(
        points,
        [
            "bk_waist_side",
            "bk_waist_cb",
            "bk_dart_mid",
            "bk_dart_dir",
            "bk_yoke_side",
            "bk_yoke_cb",
            "bk_crotch_tip",
            "ff_top",
            "bp_ref",
            "bp_f1",
            "bp_f2",
            "bp_fs2",
            "bp_fc2",
            "bp_fs1",
            "bp_fc1",
            "bp_s0",
            "bp_sl",
            "bp_sr",
            "bp_sbl",
            "bp_sbr"
        ]
    );
}
