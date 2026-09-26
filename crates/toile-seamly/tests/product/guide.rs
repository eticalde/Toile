use toile_seamly::{Evaluation, Measurements, Pattern, Product, SplineLength, import};

use super::{distance, resolve};

/// A panel whose four corners read nothing off the body, drawn with one guide
/// line whose own two places are the only readers of `across`.
///
/// A draft is full of these: the dashed line a bodice's armhole is squared off
/// against reads a width nothing else in the block does.
const PATTERN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<pattern>
    <version>0.7.4</version>
    <unit>cm</unit>
    <draftBlock name="block">
        <calculation>
            <point id="1" name="A" type="single" x="0" y="0"/>
            <point id="2" name="B" type="endLine" basePoint="1" length="30" angle="0" lineType="none"/>
            <point id="3" name="C" type="endLine" basePoint="2" length="40" angle="270" lineType="none"/>
            <point id="4" name="D" type="endLine" basePoint="1" length="40" angle="270" lineType="none"/>
            <point id="5" name="G" type="endLine" basePoint="4" length="across/2" angle="0" lineType="dashLine"/>
            <point id="6" name="H" type="endLine" basePoint="5" length="12" angle="90" lineType="dashLine"/>
        </calculation>
        <modeling>
            <point id="11" idObject="1" type="modeling"/>
            <point id="12" idObject="2" type="modeling"/>
            <point id="13" idObject="3" type="modeling"/>
            <point id="14" idObject="4" type="modeling"/>
            <point id="15" idObject="5" type="modeling"/>
            <point id="16" idObject="6" type="modeling"/>
            <path cut="false" id="17" lineType="dashLine" name="guía" type="2">
                <nodes><node idObject="15" type="NodePoint"/><node idObject="16" type="NodePoint"/></nodes>
            </path>
        </modeling>
        <pieces>
            <piece id="18" name="PANEL" seamAllowance="false" width="0" version="2">
                <data letter="A" quantity="1" onFold="false"/>
                <nodes>
                    <node idObject="11" type="NodePoint"/>
                    <node idObject="12" type="NodePoint"/>
                    <node idObject="13" type="NodePoint"/>
                    <node idObject="14" type="NodePoint"/>
                </nodes>
                <iPaths><record path="17"/></iPaths>
            </piece>
        </pieces>
    </draftBlock>
</pattern>"#;

const BODY: &str = r#"<smis>
    <unit>cm</unit>
    <body-measurements>
        <m name="across" value="20"/>
        <m name="nadie_la_lee" value="7"/>
    </body-measurements>
</smis>"#;

fn imported() -> (Pattern, Measurements, Product) {
    let pattern = Pattern::parse(PATTERN).expect("the pattern reads");
    let body = Measurements::parse(BODY).expect("the body reads");
    let product = import(&pattern, &body, "cuerpo").expect("the pattern imports");
    (pattern, body, product)
}

/// The body carries a measurement whose only reader is a guide line, and the
/// line's places resolve where the pattern puts them.
#[test]
fn a_measurement_read_only_inside_a_piece_is_carried_and_its_places_resolve() {
    let (pattern, body, product) = imported();
    let set = product.doc.measures().expect("a body");
    assert_eq!(set.get("across"), Some(20.0), "carried under its own name");
    let carried: Vec<&str> = product
        .report
        .carried
        .iter()
        .map(|m| m.seamly.as_str())
        .collect();
    assert_eq!(carried, ["across"]);

    // The guide line is drawn, and both its places are points of their own:
    // neither is a corner of the panel.
    let lines: Vec<_> = product.doc.lines.iter().map(|(_, line)| line).collect();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].label.as_deref(), Some("guía"));
    assert!(lines[0].vertices().all(|place| place.anchor().is_none()));

    // Six points, each where the pattern's own evaluation puts it. Without the
    // measurement the body would answer for nothing when the guide's two
    // points ask for it, and `resolve` would not get a number at all.
    let reference =
        Evaluation::new(&pattern, &body, SplineLength::ArcLength).expect("the pattern evaluates");
    let placed = resolve(&product.doc);
    assert_eq!(placed.len(), 6);
    for (key, at) in placed {
        let want = product.sources[&key].locate(&reference).expect("evaluated");
        assert!(distance(at, want) <= 1e-9, "{:?}", product.sources[&key]);
    }
}

/// And a measurement nothing reads stays out of the body, which is what the
/// report promises of everything it lists as left out.
#[test]
fn a_measurement_no_formula_reads_stays_out_of_the_body() {
    let (_, _, product) = imported();
    let set = product.doc.measures().expect("a body");
    assert_eq!(set.get("nadie_la_lee"), None);
    let left_out: Vec<&str> = product
        .report
        .left_out
        .iter()
        .map(|m| m.seamly.as_str())
        .collect();
    assert_eq!(left_out, ["nadie_la_lee"]);
}
