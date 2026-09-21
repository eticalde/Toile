#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_seamly::{Error, Evaluation, Formula, FormulaError, Measurements, Pattern, SplineLength};

/// A pattern whose one block holds `calculation`, then `modeling`.
fn pattern(calculation: &str, modeling: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<pattern>
    <version>0.7.4</version>
    <unit>cm</unit>
    <variables>
        <variable description="" formula="waist/4" name="#q"/>
    </variables>
    <draftBlock name="block">
        <calculation>
            <point id="1" name="A" type="single" x="0" y="0"/>
            {calculation}
        </calculation>
        <modeling>{modeling}</modeling>
        <pieces/>
    </draftBlock>
</pattern>"##
    )
}

const BODY: &str = r#"<smis>
    <unit>cm</unit>
    <personal>
        <given-name>SENTINEL-NAME</given-name>
        <email>sentinel@example.org</email>
    </personal>
    <body-measurements>
        <m name="waist" value="80"/>
        <m name="half" value="waist/2"/>
    </body-measurements>
</smis>"#;

fn unsupported(error: &Error) -> String {
    assert!(matches!(error, Error::Unsupported { .. }), "{error}");
    error.to_string()
}

#[test]
fn an_unsupported_point_type_is_an_error_naming_the_object() {
    let xml = pattern(
        r#"<point id="7" name="X" type="normal" basePoint="1"/>"#,
        "",
    );
    let message = unsupported(&Pattern::parse(&xml).unwrap_err());
    for needle in ["line 11", "type=\"normal\"", "id=\"7\"", "name=\"X\""] {
        assert!(message.contains(needle), "{message}");
    }
}

#[test]
fn an_attribute_outside_the_model_is_an_error_naming_it() {
    let xml = pattern(
        r#"<point id="2" name="B" type="endLine" basePoint="1" length="1" angle="0" kAsm1="2"/>"#,
        "",
    );
    let message = unsupported(&Pattern::parse(&xml).unwrap_err());
    assert!(message.contains("`kAsm1`"), "{message}");
}

#[test]
fn a_reference_to_an_object_below_is_malformed() {
    let xml = pattern(
        r#"<point id="2" name="B" type="endLine" basePoint="3" length="1" angle="0"/>
           <point id="3" name="C" type="single" x="1" y="1"/>"#,
        "",
    );
    let error = Pattern::parse(&xml).unwrap_err();
    assert!(matches!(error, Error::Malformed { .. }), "{error}");
    assert!(
        error.to_string().contains("id 3 is not defined above"),
        "{error}"
    );
}

#[test]
fn a_modeling_copy_must_copy_an_object_of_its_kind() {
    let xml = pattern(
        "",
        r#"<spline id="9" idObject="1" inUse="true" type="modelingSpline"/>"#,
    );
    let error = Pattern::parse(&xml).unwrap_err();
    assert!(
        error.to_string().contains("id 1 is a point, not a spline"),
        "{error}"
    );
}

#[test]
fn an_unknown_modeling_object_is_an_error_naming_it() {
    let xml = pattern("", r#"<point id="9" idObject="1" type="mirror"/>"#);
    let message = unsupported(&Pattern::parse(&xml).unwrap_err());
    assert!(message.contains("type=\"mirror\""), "{message}");
}

#[test]
fn a_formula_citing_a_name_nothing_defines_is_an_error_naming_the_point() {
    let xml = pattern(
        r#"<point id="2" name="B" type="endLine" basePoint="1" length="Line_A_Z" angle="0"/>"#,
        "",
    );
    let body = Measurements::parse(BODY).unwrap();
    let error = Evaluation::new(
        &Pattern::parse(&xml).unwrap(),
        &body,
        SplineLength::ArcLength,
    )
    .unwrap_err();
    let Error::Formula { at, source, .. } = error else {
        panic!("{error}");
    };
    assert!(at.element.contains("name=\"B\""));
    assert_eq!(source, FormulaError::UnknownName("Line_A_Z".to_owned()));
}

#[test]
fn a_drawn_line_defines_its_length_and_heading_both_ways() {
    let xml = pattern(
        r##"<point id="2" name="B" type="endLine" basePoint="1" length="#q" angle="90" lineType="none"/>
           <line firstPoint="1" id="3" lineType="none" secondPoint="2"/>
           <point id="4" name="C" type="endLine" basePoint="2" length="Line_B_A" angle="AngleLine_B_A"/>"##,
        "",
    );
    let body = Measurements::parse(BODY).unwrap();
    let evaluation = Evaluation::new(
        &Pattern::parse(&xml).unwrap(),
        &body,
        SplineLength::ArcLength,
    )
    .unwrap();
    assert_eq!(evaluation.point_named("B"), Some([0.0, -20.0]));
    assert_eq!(evaluation.drawn("AngleLine_A_B"), Some(90.0));
    assert_eq!(evaluation.drawn("AngleLine_B_A"), Some(270.0));
    assert_eq!(evaluation.point_named("C"), Some([0.0, 0.0]));
}

#[test]
fn the_personal_block_is_never_read_and_measurements_resolve_in_order() {
    let body = Measurements::parse(BODY).unwrap();
    assert_eq!(body.get("half"), Some(40.0));
    let read = format!("{body:?}");
    assert!(
        !read.contains("SENTINEL") && !read.contains("sentinel"),
        "{read}"
    );
}

#[test]
fn a_measurement_citing_one_below_it_is_an_error() {
    let body = BODY.replace(
        r#"<m name="waist" value="80"/>"#,
        r#"<m name="waist" value="half*2"/>"#,
    );
    let error = Measurements::parse(&body).unwrap_err();
    assert!(matches!(error, Error::Formula { .. }), "{error}");
}

#[test]
fn the_formula_grammar_is_the_arithmetic_the_files_use_and_no_more() {
    let formula = Formula::parse("(hip/2/10)+1 - -#x*CurrentLength").unwrap();
    let lookup = |name: &str| match name {
        "hip" => Ok(98.0),
        "#x" => Ok(2.0),
        "CurrentLength" => Ok(3.0),
        other => Err(FormulaError::UnknownName(other.to_owned())),
    };
    assert_eq!(
        formula.eval(&lookup),
        Ok((98.0 / 2.0 / 10.0) + 1.0 - -2.0 * 3.0)
    );
    assert_eq!(formula.names().len(), 3);
    for (source, what) in [
        ("sqrt(4)", "the function call `sqrt(`"),
        ("2^3", "the power `^`"),
        ("1,5", "an argument list"),
        ("a<b?1:2", "a comparison"),
    ] {
        assert_eq!(
            Formula::parse(source),
            Err(FormulaError::Unsupported(what.to_owned())),
            "{source}"
        );
    }
    for source in ["", "1 +", "(1", "1 2", "1.2.3", "\u{2212}1"] {
        assert!(
            matches!(Formula::parse(source), Err(FormulaError::Syntax { .. })),
            "{source}"
        );
    }
    let zero = Formula::parse("1/0").unwrap();
    assert_eq!(zero.eval(&lookup), Err(FormulaError::NotFinite));
}
