use toile_engine::draft::{Command, Doc, Draft, LineKind, LineVertex, NotchCount, PointKey};
use toile_seamly::{Evaluation, Measurements, SplineLength, seamly_measurement};

use super::super::check::{GROWTH, PARITY};
use super::{inputs, product};

/// The whole trip a line makes: written into the file, read back by the reader
/// that ships, resolved by the engine, and landing where the Seamly path's own
/// evaluator puts it — on the owner's body, and on one grown in hip and inseam.
///
/// Every other check of the lines runs on the product in memory or through the
/// crate's own resolver. This one is the one that would catch a place the file
/// cannot carry or the engine cannot resolve.
#[test]
fn the_written_product_reopens_and_its_lines_resolve_where_the_paths_do() {
    let product = product();
    let (pattern, body) = inputs();
    let text = product.doc.to_canonical_json();
    let doc = Doc::from_json(&text).expect("the product is a Toile file");
    assert_eq!(doc.lines.iter().count(), 25);
    let mut draft = Draft::from_doc(doc).expect("the product resolves");
    let mut counted = [0_usize; 2];
    let mut worst = [0.0_f64; 2];
    for grown in [false, true] {
        let seamly = if grown {
            wider(&mut draft, &body)
        } else {
            body.clone()
        };
        let evaluated = Evaluation::new(&pattern, &seamly, SplineLength::ArcLength)
            .expect("the pattern evaluates");
        for key in line_places(&draft) {
            let source = product.sources[&key];
            let want = source.locate(&evaluated).expect("the path evaluates");
            let at = draft.resolved(key).expect("the engine places it");
            let gap = (want[0] - at[0]).hypot(want[1] - at[1]);
            let follows = !grown || !product.frozen.contains_key(&key);
            let bound = if follows { PARITY } else { 3.5 };
            assert!(gap <= bound, "{source:?} lands {gap:e} cm off");
            counted[usize::from(!follows)] += 1;
            worst[usize::from(!follows)] = worst[usize::from(!follows)].max(gap);
        }
    }
    assert_eq!(counted, [159, 35]);
    println!(
        "reopened: {} places within {:e} cm, {} on something frozen, worst {:.4} cm",
        counted[0], worst[0], counted[1], worst[1]
    );
}

/// Every point the internal lines stand on, in line order: the node an anchored
/// place names, each free place, and each handle a curved span hangs on.
fn line_places(draft: &Draft) -> Vec<PointKey> {
    let mut out = Vec::new();
    for (_, line) in draft.doc().lines.iter() {
        for place in line.vertices() {
            match place {
                LineVertex::Contour(anchor) => out.push(anchor.from),
                LineVertex::Free { point } => out.push(point),
            }
        }
        out.extend(line.handles());
    }
    out
}

/// The draft's body grown as the report grows it, and the same growth as a
/// Seamly body for the pattern's own evaluator.
fn wider(draft: &mut Draft, body: &Measurements) -> Measurements {
    let mut seamly = body.clone();
    let mannequin = draft.doc().resolve_with;
    for (toile, by) in GROWTH {
        let name = seamly_measurement(toile).expect("the mapping names both");
        let value = body.get(name).expect("the owner's body carries it") + by;
        seamly = seamly.with_value(name, value).expect("the body has it");
        draft
            .edit(Command::SetMeasure {
                mannequin,
                name: toile.to_owned(),
                to: value,
            })
            .expect("the body carries the measure");
    }
    seamly
}

/// The drawing carries one path per internal line beside the ten cut lines,
/// each named after the line, and the dashed ones are the ones drawn dashed.
///
/// A drawing that lost them would be a sheet of paper nobody can cut a pair of
/// jeans from, which is the whole reason the lines were carried over.
pub(super) fn drawn_lines(svg: &str, doc: &Doc) {
    // A grain line is an arrow, and an arrow is a shaft and four barbs.
    const GRAIN: usize = 5;

    assert!(svg.starts_with("<?xml"), "{svg}");
    let paths = svg.matches("<path ").count();
    let pieces = doc.pieces.len();
    let single = doc
        .notches
        .iter()
        .filter(|(_, notch)| notch.count == NotchCount::Single)
        .count();
    assert_eq!(doc.lines.len(), 25);
    assert_eq!(single, doc.notches.len(), "the draft marks single notches");
    assert_eq!(
        paths,
        pieces * (1 + GRAIN) + doc.lines.len() + single,
        "a cut line and a grain arrow each, every drawn line, and every notch"
    );
    for (_, line) in doc.lines.iter() {
        let name = line.label.as_deref().expect("the author named it");
        assert!(svg.contains(&format!("<title>{name}</title>")), "{name}");
    }
    let dashed = doc
        .lines
        .iter()
        .filter(|(_, line)| line.kind == LineKind::Reference)
        .count();
    assert_eq!(svg.matches("stroke-dasharray").count(), dashed);
    assert_eq!(dashed, 8);
}
