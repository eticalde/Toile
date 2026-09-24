use toile_doc::{LineKind, Winding};
use toile_seamly::{
    Carried, Evaluation, Frozen, Measurements, Pattern, Refusal, SplineLength, import,
};

use super::{distance, resolve};

/// A block with a curve, a point cut on it used as a corner, and an arc of
/// two hundred degrees closed by its chord: what the owner's file does not
/// exercise.
const PATTERN: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<pattern>
    <version>0.7.4</version>
    <unit>cm</unit>
    <variables>
        <variable description="a quarter" formula="waist/4" name="#Q"/>
    </variables>
    <draftBlock name="block">
        <calculation>
            <point id="1" name="A" type="single" x="0" y="0"/>
            <point id="2" name="B" type="endLine" basePoint="1" length="#Q" angle="0" lineType="none"/>
            <point id="3" name="C" type="endLine" basePoint="2" length="10" angle="270" lineType="none"/>
            <spline id="4" type="simpleInteractive" point1="1" point4="3" angle1="270" length1="8" angle2="180" length2="8"/>
            <point id="5" name="K" type="cutSpline" spline="4" length="Spl_A_C/3"/>
            <point id="6" name="O" type="endLine" basePoint="1" length="#Q+10" angle="270" lineType="none"/>
            <arc id="7" type="simple" center="6" radius="3" angle1="0" angle2="200"/>
        </calculation>
        <modeling>
            <point id="11" idObject="1" type="modeling"/>
            <point id="12" idObject="2" type="modeling"/>
            <point id="13" idObject="3" type="modeling"/>
            <point id="15" idObject="5" type="modeling"/>
            <arc id="17" idObject="7" type="modeling"/>
            <path cut="false" id="18" lineType="solidLine" name="marca sola" type="2">
                <nodes><node idObject="12" type="NodePoint"/></nodes>
            </path>
            <path cut="true" id="19" lineType="dashLine" name="abertura" type="2">
                <nodes><node idObject="11" type="NodePoint"/><node idObject="13" type="NodePoint"/></nodes>
            </path>
        </modeling>
        <pieces>
            <piece id="20" name="CUT" seamAllowance="false" width="0" version="2">
                <data letter="A" quantity="1" onFold="false"/>
                <nodes>
                    <node idObject="11" type="NodePoint"/>
                    <node idObject="12" type="NodePoint"/>
                    <node idObject="13" type="NodePoint"/>
                    <node idObject="15" type="NodePoint"/>
                </nodes>
                <iPaths><record path="18"/><record path="19"/></iPaths>
            </piece>
            <piece id="21" name="ARC" seamAllowance="false" width="0" version="2">
                <data letter="B" quantity="1" onFold="false"/>
                <nodes>
                    <node idObject="17" type="NodeArc" reverse="0"/>
                </nodes>
            </piece>
        </pieces>
    </draftBlock>
</pattern>"##;

const BODY: &str = r#"<smis>
    <unit>cm</unit>
    <body-measurements>
        <m name="waist" value="80"/>
    </body-measurements>
</smis>"#;

fn inputs(waist: f64) -> (Pattern, Measurements) {
    let pattern = Pattern::parse(PATTERN).expect("the pattern reads");
    let body = Measurements::parse(BODY)
        .expect("the body reads")
        .with_value("waist", waist)
        .expect("it has a waist");
    (pattern, body)
}

#[test]
fn a_cut_corner_follows_its_curve_at_a_frozen_parameter_and_an_arc_is_cut_in_spans() {
    let (pattern, body) = inputs(80.0);
    let mut product = import(&pattern, &body, "sintético").expect("the pattern imports");
    let set = product.doc.measures().expect("a body");
    assert_eq!(set.get("waist"), Some(80.0), "carried under its own name");
    assert_eq!(product.report.carried.len(), 1);

    let arc = product.doc.piece_named("ARC").expect("imported");
    let held = product.doc.pieces.get(arc).expect("live");
    assert_eq!(held.contour.len(), 4, "three spans and the chord");
    assert_eq!(held.winding, Winding::Ccw);

    let reference = Evaluation::new(&pattern, &body, SplineLength::ArcLength).expect("evaluates");
    for (key, at) in resolve(&product.doc) {
        let want = product.sources[&key].locate(&reference).expect("evaluated");
        assert!(distance(at, want) <= 1e-9, "{:?}", product.sources[&key]);
    }

    // A wider body: everything follows but the cut, which stays on the curve
    // at the parameter it was frozen at, off the arc length it asked for.
    let (_, wider) = inputs(96.0);
    let mannequin = product.doc.resolve_with;
    toile_doc::Command::SetMeasure {
        mannequin,
        name: "waist".to_owned(),
        to: 96.0,
    }
    .apply(&mut product.doc)
    .expect("the body carries it");
    let grown = Evaluation::new(&pattern, &wider, SplineLength::ArcLength).expect("evaluates");
    let t = product.report.frozen[0].value;
    assert_eq!(product.report.frozen[0].frozen, Frozen::CutParameter(5));
    assert_eq!(product.report.frozen[0].reaches, ["K"]);
    for (key, at) in resolve(&product.doc) {
        let want = product.sources[&key].locate(&grown).expect("evaluated");
        if product.frozen.contains_key(&key) {
            let on_curve = grown.splines[&4].point(t);
            assert!(distance(at, on_curve) <= 1e-9, "the cut stays on its curve");
            assert!(distance(at, want) > 1e-6, "and drifts off its arc length");
        } else {
            assert!(distance(at, want) <= 1e-9, "{:?}", product.sources[&key]);
        }
    }
}

/// A path the product cannot draw is said out loud, and one the file says the
/// cutter opens the cloth along becomes a slit whose two places are corners.
#[test]
fn a_path_of_one_place_is_reported_and_a_cut_one_is_a_slit_between_two_corners() {
    let (pattern, body) = inputs(80.0);
    let product = import(&pattern, &body, "sintético").expect("the pattern imports");
    let cut = product.doc.piece_named("CUT").expect("imported");
    let notes = &product
        .report
        .pieces
        .iter()
        .find(|note| note.name == "CUT")
        .expect("reported")
        .internal;
    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0].name, "marca sola");
    assert_eq!(notes[0].carried, Carried::Refused(Refusal::OnePlace));
    assert_eq!(
        notes[1].carried,
        Carried::Line {
            kind: LineKind::Slit,
            places: 2,
            anchored: 2,
            curves: 0,
            stray: 0.0,
        }
    );

    // Only the second one is in the document, and it is drawn on nothing but
    // the contour: a slit between two corners adds no point at all.
    let lines: Vec<_> = product.doc.lines.iter().map(|(_, line)| line).collect();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].label.as_deref(), Some("abertura"));
    assert_eq!(lines[0].piece, cut);
    assert!(lines[0].kind.opens_the_cloth());
    let anchors: Vec<_> = lines[0]
        .vertices()
        .map(|place| place.anchor().expect("both places are corners"))
        .collect();
    assert!(anchors.iter().all(|anchor| anchor.piece == cut));
    let held = product.doc.pieces.get(cut).expect("live");
    let labels: Vec<Option<String>> = anchors
        .iter()
        .map(|anchor| product.doc.label_of(cut, anchor.from))
        .collect();
    assert_eq!(
        labels,
        [Some("A".to_owned()), Some("C".to_owned())],
        "{held:?}"
    );
}
