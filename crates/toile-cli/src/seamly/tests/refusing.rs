use std::collections::BTreeMap;
use std::path::PathBuf;

use toile_engine::draft::{Binding, LineVertex, PointKey};
use toile_seamly::Product;

use super::super::args::Asked;
use super::super::check::Check;
use super::super::{migrate, refused};
use super::{check, inputs, product};

/// A check of a product that came out perfect, for a test to spoil one field
/// of.
fn clean() -> Check {
    Check {
        points: 164,
        worst: 0.0,
        grown: Vec::new(),
        followed: (164, 0.0),
        drift: BTreeMap::new(),
        defects: Vec::new(),
        unplaced: Vec::new(),
        trusting: None,
    }
}

#[test]
fn a_product_whose_every_point_lands_on_the_pattern_is_written() {
    assert_eq!(refused(&clean()), None);
}

#[test]
fn a_piece_that_resolves_to_a_shape_nobody_can_cut_is_refused_with_the_defect() {
    let mut broken = clean();
    broken.defects = vec!["«DELANTERO»: el contorno se cruza".to_owned()];
    let why = refused(&broken).expect("refused");
    assert!(why.contains("no resuelve limpio"), "{why}");
    assert!(why.contains("el contorno se cruza"), "{why}");
}

/// The refusal that «peor inf cm» used to be printed instead of: the points are
/// named, so the reader knows which part of the draft to look at.
#[test]
fn points_that_land_nowhere_are_refused_by_name_and_counted_against_the_whole() {
    let mut nowhere = clean();
    nowhere.unplaced = vec!["ancho_espalda".to_owned(), "ancho_espalda_up".to_owned()];
    let why = refused(&nowhere).expect("refused");
    assert!(why.contains("2 de sus 164 puntos no caen"), "{why}");
    assert!(why.contains("ancho_espalda, ancho_espalda_up"), "{why}");
}

/// And the check finds one the defects cannot.
///
/// A defect is a piece's, and a piece walks its own contour: the place of an
/// internal line is on no contour, so a line's place resolving nowhere was
/// nobody's defect. The line is left off the drawing, the parity comes out
/// infinite, and before this the run said so and exited zero.
#[test]
fn a_line_place_that_resolves_nowhere_is_no_piece_defect_and_is_caught_anyway() {
    let mut product = product();
    let key = free_place(&product);
    let point = product.doc.points.get_mut(key).expect("a live point");
    point.x = Binding::parse("ninguna_medida").expect("a bare name is a formula");
    let named = point.label.clone().expect("the path names its places");

    let (pattern, measured) = inputs();
    let found = check(&product, &pattern, &measured).expect("the document still opens");
    assert!(found.defects.is_empty(), "{:?}", found.defects);
    assert!(!found.worst.is_finite(), "{} cm", found.worst);
    assert_eq!(found.unplaced, [named.as_str()]);
    let why = refused(&found).expect("refused");
    assert!(why.contains("1 de sus 164 puntos no cae"), "{why}");
    assert!(why.contains(&named), "{why}");
}

/// A pattern whose one piece walks its four corners crosswise, so the contour
/// is a bowtie: it imports, and no shape a person could cut comes out of it.
const CROSSED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<pattern>
    <version>0.7.4</version>
    <unit>cm</unit>
    <measurements>cuerpo.smis</measurements>
    <draftBlock name="block">
        <calculation>
            <point id="1" name="A" type="single" x="0" y="0"/>
            <point id="2" name="B" type="endLine" basePoint="1" length="ancho" angle="0" lineType="hair"/>
            <point id="3" name="C" type="endLine" basePoint="2" length="40" angle="270" lineType="hair"/>
            <point id="4" name="D" type="endLine" basePoint="1" length="40" angle="270" lineType="hair"/>
        </calculation>
        <modeling>
            <point id="11" idObject="1" type="modeling"/>
            <point id="12" idObject="2" type="modeling"/>
            <point id="13" idObject="3" type="modeling"/>
            <point id="14" idObject="4" type="modeling"/>
        </modeling>
        <pieces>
            <piece id="18" name="CRUZ" seamAllowance="false" width="0" version="2">
                <data letter="A" quantity="1" onFold="false"/>
                <nodes>
                    <node idObject="11" type="NodePoint"/>
                    <node idObject="12" type="NodePoint"/>
                    <node idObject="14" type="NodePoint"/>
                    <node idObject="13" type="NodePoint"/>
                </nodes>
            </piece>
        </pieces>
    </draftBlock>
</pattern>"#;

const CROSSED_BODY: &str = r#"<smis>
    <unit>cm</unit>
    <body-measurements>
        <m name="ancho" value="30"/>
    </body-measurements>
</smis>"#;

/// The pattern and its body beside each other, as the command reads them.
fn crossed_folder() -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("toile-cruz-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("a scratch folder");
    std::fs::write(root.join("cuerpo.smis"), CROSSED_BODY).expect("written");
    let input = root.join("cruz.sm2d");
    std::fs::write(&input, CROSSED).expect("written");
    (root, input)
}

/// The refusal is `migrate`'s, not only `refused`'s.
///
/// Every test above this one calls `refused` itself, so all of them pass with
/// the command never asking it: what a person sees is the exit code and an
/// empty folder, and both come from the one line in `migrate` that consults it.
#[test]
fn a_product_the_check_refuses_leaves_nothing_on_disk() {
    let (root, input) = crossed_folder();
    let output = input.with_extension("toile");
    let asked = Asked {
        input: input.clone(),
        output: output.clone(),
        persona: None,
    };
    let why = migrate(&asked).expect_err("the panel crosses itself");
    assert!(why.contains("no resuelve limpio"), "{why}");
    assert!(why.contains("CRUZ"), "{why}");
    for beside in [
        output,
        input.with_extension("svg"),
        input.with_file_name("cruz - importado a Toile.md"),
    ] {
        assert!(!beside.exists(), "{} was written", beside.display());
    }
    std::fs::remove_dir_all(root).expect("the scratch folder goes");
}

/// A point an internal line stands on and no piece's contour does.
fn free_place(product: &Product) -> PointKey {
    product
        .doc
        .lines
        .iter()
        .flat_map(|(_, line)| line.vertices())
        .filter_map(|place| match place {
            LineVertex::Free { point } => Some(point),
            LineVertex::Contour(_) => None,
        })
        .find(|&point| {
            product.doc.pieces_citing(point).is_empty()
                && product
                    .doc
                    .points
                    .get(point)
                    .is_some_and(|held| held.label.is_some())
        })
        .expect("the owner's jeans draw their pockets off points of their own")
}
