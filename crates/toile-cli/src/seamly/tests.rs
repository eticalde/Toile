use std::path::PathBuf;

use toile_engine::body::Collider;
use toile_engine::draft::{Doc, Draft};
use toile_engine::session::Session;
use toile_seamly::{Frozen, Measurements, Pattern, Product, import};

use self::inner::drawn_lines;
use super::args::Asked;
use super::check::{PARITY, check};
use super::migrate;

/// The internal lines, from the written file through the engine to the drawing.
mod inner;
/// Filing the body as a person in the library, and the product's link.
mod persona;
/// Why a checked product is not written.
mod refusing;

// The owner's pattern and body, as the reader's own tests keep them.
const PATTERN: &str = include_str!("../../../toile-seamly/tests/fixtures/baggy-jeans.sm2d");
const BODY: &str = include_str!("../../../toile-seamly/tests/fixtures/measurements-eti.smis");

fn inputs() -> (Pattern, Measurements) {
    let pattern = Pattern::parse(PATTERN).expect("the owner's pattern reads");
    let body = Measurements::parse(BODY).expect("the owner's measurements read");
    (pattern, body)
}

fn product() -> Product {
    let (pattern, body) = inputs();
    import(&pattern, &body, "measurements-eti").expect("the owner's pattern imports")
}

#[test]
fn the_engine_resolves_every_point_where_the_pattern_puts_it_and_every_piece_clean() {
    let product = product();
    let (pattern, body) = inputs();
    let found = check(&product, &pattern, &body).expect("the product resolves");
    assert!(found.defects.is_empty(), "{:?}", found.defects);
    assert_eq!(found.points, product.doc.points.len());
    assert!(found.worst <= PARITY, "{} cm", found.worst);
    let draft = Draft::from_doc(product.doc.clone()).expect("the product resolves");
    for (key, _) in product.doc.pieces.iter() {
        assert!(draft.points_cm(key).len() >= 3);
        assert!(draft.perimeter_cm(key) > 0.0);
    }
}

#[test]
fn a_grown_body_moves_everything_but_the_frozen_excess_exactly_as_the_pattern() {
    let product = product();
    let (pattern, body) = inputs();
    let found = check(&product, &pattern, &body).expect("the product resolves");
    assert_eq!(
        found.grown,
        [
            ("cadera".to_owned(), 102.0),
            ("entrepierna".to_owned(), 83.5)
        ]
    );
    assert!(found.followed.1 <= PARITY, "{} cm", found.followed.1);
    assert_eq!(
        found.followed.0 + product.frozen.len(),
        product.doc.points.len()
    );
    let excess = found
        .drift
        .iter()
        .find(|(frozen, _)| matches!(frozen, Frozen::SplineExcess(_)))
        .map(|(_, drift)| *drift)
        .expect("the back yoke hangs on one");
    assert!(excess > PARITY && excess < 0.01, "{excess} cm");
    let hook = found
        .drift
        .iter()
        .find(|(frozen, _)| matches!(frozen, Frozen::CutParameter(_)))
        .map(|(_, drift)| *drift)
        .expect("the fly's cut point is reported");
    assert!(hook.is_finite() && hook > PARITY, "{hook} cm");
    println!("drift: {:?}", found.drift);
}

#[test]
fn the_product_file_reads_back_to_the_same_bytes() {
    let written = product().doc.to_canonical_json();
    let read = Doc::from_json(&written).expect("a Toile file");
    assert_eq!(read.to_canonical_json(), written);
}

#[test]
fn the_product_opens_in_a_session() {
    let session = Session::from_doc(product().doc, Collider::demo()).expect("the product opens");
    assert_eq!(session.pieces().len(), 10);
}

#[test]
fn an_evaluator_trusting_the_written_lengths_misplaces_the_back_waist() {
    let product = product();
    let (pattern, body) = inputs();
    let found = check(&product, &pattern, &body).expect("the product resolves");
    let trusting = found.trusting.expect("every spline writes a length");
    // A stale written length reaches the back waist and its dart, the back
    // yoke, the internal lines drawn off that yoke — the flap, the pocket slit
    // and the bag guide — and the fly's cut point, which the fly topstitch is
    // drawn to.
    assert_eq!(trusting.points.len(), 32, "{:?}", trusting.points);
    for label in [
        "bk_dart_tip",
        "bk_waist_side",
        "bk_yoke_cb",
        "ff_hook",
        "bp_stl",
        "bp_bgr",
    ] {
        assert!(trusting.points.iter().any(|at| at == label), "{label}");
    }
    assert!(
        trusting.worst > 5e-4 && trusting.worst < 7e-4,
        "{}",
        trusting.worst
    );
    let grown = trusting.grown.expect("the grown body evaluates");
    assert!(grown > 1.0 && grown < 5.0, "{grown} cm");
    println!("trusting: {} cm here, {grown} cm grown", trusting.worst);
}

/// A folder laid out as the owner's is, the pattern naming its body by a
/// path relative to itself.
fn folder(test: &str) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("toile-seamly-{}-{test}", std::process::id()));
    let patterns = root.join("patterns");
    let bodies = root.join("measurements/individual");
    std::fs::create_dir_all(&patterns).expect("a scratch folder");
    std::fs::create_dir_all(&bodies).expect("a scratch folder");
    std::fs::write(bodies.join("measurements-eti.smis"), BODY).expect("written");
    let input = patterns.join("Baggy Jeans [Muller].sm2d");
    std::fs::write(&input, PATTERN).expect("written");
    (root, input)
}

#[test]
fn migrating_writes_the_product_its_report_and_its_drawing_and_never_overwrites() {
    let (root, input) = folder("migrate");
    let output = input.with_extension("toile");
    let asked = Asked {
        input: input.clone(),
        output: output.clone(),
        persona: None,
    };
    let lines = migrate(&asked).expect("the owner's pattern migrates");
    let summary = lines.first().expect("a first line").clone();
    assert!(summary.starts_with("10 piezas"), "{summary}");
    assert!(summary.contains("25 líneas internas"), "{summary}");
    // What the pieces say about being cut out is the newest thing the product
    // carries, so it is the version the file asks for.
    assert!(summary.contains("formato 10"), "{summary}");
    let report = input.with_file_name("Baggy Jeans [Muller] - importado a Toile.md");
    let svg = input.with_extension("svg");
    let text = std::fs::read_to_string(&report).expect("the report is written");
    for needle in [
        "## Lo congelado",
        "Spl_crotch_waist_margin_up",
        "ff_hook",
        "`#PhL` | `phl`",
        "con el nombre del archivo de medidas",
        "### Notas del patrón (26)",
        "sin contar la firma que Seamly",
        "## Líneas internas (25)",
        "No quedó ningún trayecto interno fuera del producto.",
        "## Cómo se corta cada pieza",
        "| PRETINA | C | 1 | 1.5 | vertical | PRETINA / cortar 1 al doblez c.b. |",
        "| SOLAPA TRASERA | J | 2 | 1 | vertical | SOLAPA TRASERA / cortar 2 + entretela |",
        "| TIRA PASACINTOS | F | 1 | neta | vertical |",
        "Ninguna de las 10 piezas escribe un ángulo de hilo",
    ] {
        assert!(text.contains(needle), "the report lacks {needle}");
    }
    // The report is what the owner reads after importing, so a sentence that
    // outlived what it described is worse than no sentence.
    for gone in ["El producto va neto", "ni un ángulo escrito", "van netas"] {
        assert!(!text.contains(gone), "the report still says «{gone}»");
    }
    stale_lengths(&text);
    comments_by_place_only(&text);
    // One row per path in the table of lines, each naming its Seamly stroke.
    assert_eq!(
        text.matches("| continuo |").count() + text.matches("| discontinuo |").count(),
        25
    );
    let written = std::fs::read_to_string(&output).expect("the product is written");
    let doc = Doc::from_json(&written).expect("a Toile file");
    assert_eq!((doc.pieces.len(), doc.lines.iter().count()), (10, 25));
    let drawing = std::fs::read_to_string(&svg).expect("drawn");
    drawn_lines(&drawing, &doc);
    cut_out(&drawing);
    let again = migrate(&asked).expect_err("nothing is written twice");
    assert!(again.contains("ya existe"), "{again}");
    assert_eq!(
        std::fs::read_to_string(&output).expect("still there"),
        written
    );
    std::fs::remove_dir_all(root).expect("the scratch folder goes");
}

/// What the owner's own pieces say about being cut out, in the drawing written
/// beside the product.
///
/// Three of his ten: the panel cut in mirrored pairs, the strip cut net — the
/// question he went back to his notes for twice — and the flap whose label and
/// whose count disagree, where the drawing shows both without choosing.
fn cut_out(drawing: &str) {
    for said in [
        ">A · «DELANTERO»</text>",
        ">Cortar 2 · margen 1.5 cm por fuera del contorno</text>",
        ">cortar 2 espejadas</text>",
        ">F · «TIRA PASACINTOS»</text>",
        ">Cortar 1 · sin margen: corta por el contorno</text>",
        ">J · «SOLAPA TRASERA»</text>",
        ">Cortar 2 · margen 1 cm por fuera del contorno</text>",
        ">cortar 2 + entretela</text>",
    ] {
        assert!(drawing.contains(said), "the drawing lacks {said}");
    }
}

/// The report's table of stale lengths lists the six splines whose written
/// length the curve has left behind, and none of the five that still agree.
fn stale_lengths(text: &str) {
    let start = text
        .find("## Largos de curva guardados")
        .expect("the section");
    let section = &text[start..];
    let section = &section[..section[2..]
        .find("\n## ")
        .map_or(section.len(), |end| end + 2)];
    let mut listed: Vec<&str> = section
        .lines()
        .filter_map(|line| line.strip_prefix("| `"))
        .filter_map(|line| line.split('`').next())
        .collect();
    listed.sort_unstable();
    assert_eq!(
        listed,
        [
            "Spl_bk_crotch_tip_B3",
            "Spl_bk_crotch_tip_b_knee_back_end",
            "Spl_crotch_waist_margin_up",
            "Spl_ff_top_ff_hook",
            "Spl_hip_end_A1",
            "Spl_waist_margin_up_waist_margin_end",
        ]
    );
    assert!(section.contains("En 6 de las 11 curvas"), "{section}");
    assert!(section.contains("`bk_waist_side`"), "{section}");
}

/// The pattern's comments are counted and placed, and not one line of what
/// they say reaches the report.
fn comments_by_place_only(text: &str) {
    let said: Vec<&str> = PATTERN
        .split("<!--")
        .skip(1)
        .filter_map(|rest| rest.split("-->").next())
        .flat_map(str::lines)
        .map(str::trim)
        .filter(|line| line.len() > 12)
        .collect();
    assert!(said.len() > 26, "{}", said.len());
    for line in said {
        assert!(!text.contains(line), "the report quotes `{line}`");
    }
}
