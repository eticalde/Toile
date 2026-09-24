use std::collections::BTreeMap;

use toile_doc::{LineKind, LineVertex};
use toile_seamly::Carried;

use super::{body, distance, product, reference, resolve};

/// The twenty-five paths the owner drew inside his pieces, every one of them
/// now a line of the product, on the piece that cites it.
#[test]
fn every_internal_path_of_the_owner_pattern_is_a_line_of_the_piece_that_draws_it() {
    let product = product();
    assert_eq!(product.doc.lines.iter().count(), 25);
    let mut by_piece: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (_, line) in product.doc.lines.iter() {
        let piece = product.doc.pieces.get(line.piece).expect("a live piece");
        let label = line.label.as_deref().expect("the author named it");
        by_piece.entry(piece.name.as_str()).or_default().push(label);
    }
    assert_eq!(
        by_piece,
        BTreeMap::from([
            (
                "DELANTERO",
                vec![
                    "ranura pill",
                    "canal celular",
                    "pespunte bragueta",
                    "quiebre"
                ]
            ),
            (
                "TRASERO + CANESU",
                vec!["linea canesu", "solapa", "ranura pill tras", "guia bolsa"]
            ),
            (
                "PRETINA",
                vec![
                    "pasacintos 1",
                    "pasacintos 2",
                    "pasacintos 3",
                    "pasacintos 4",
                    "pasacintos 5",
                    "pasacintos 6",
                    "pasacintos 7"
                ]
            ),
            ("VISTA BRAGUETA", vec!["ojal 1", "ojal 2", "ojal 3"]),
            (
                "PANEL BOTONES",
                vec!["doblez", "boton 1", "boton 2", "boton 3"]
            ),
            (
                "TIRA CADENA",
                vec!["corte eslabon 1", "corte eslabon 2", "bies 45"]
            ),
        ])
    );
}

/// The stroke is all Seamly says, so it is all the kind is read from — and
/// neither kind it can give takes cloth out of a piece.
#[test]
fn a_solid_stroke_becomes_a_placement_and_a_dashed_one_a_reference() {
    let product = product();
    for note in product.report.pieces.iter().flat_map(|p| &p.internal) {
        let Carried::Line { kind, .. } = note.carried else {
            panic!("`{}` was not drawn", note.name);
        };
        let wanted = match note.line_type.as_str() {
            "solidLine" => LineKind::Placement,
            _ => LineKind::Reference,
        };
        assert_eq!(kind, wanted, "{}", note.name);
        assert!(!kind.opens_the_cloth(), "{}", note.name);
        assert!(!note.cut, "the owner cuts no piece along an internal path");
    }
    let count = |kind: LineKind| {
        product
            .doc
            .lines
            .iter()
            .filter(|(_, line)| line.kind == kind)
            .count()
    };
    assert_eq!(
        (count(LineKind::Placement), count(LineKind::Reference)),
        (17, 8)
    );
    assert_eq!(count(LineKind::Slit), 0);
}

/// What the lines are made of: the places, the curves, and the one place of the
/// twenty-five paths that lands on a corner of its own piece.
#[test]
fn the_places_and_the_curves_are_the_ones_the_paths_walk() {
    let product = product();
    let notes: Vec<_> = product
        .report
        .pieces
        .iter()
        .flat_map(|p| &p.internal)
        .collect();
    assert_eq!(notes.len(), 25);
    let mut places = 0;
    let mut curves = 0;
    let mut anchored = Vec::new();
    for note in &notes {
        let Carried::Line {
            places: walked,
            anchored: on_contour,
            curves: bends,
            stray,
            ..
        } = note.carried
        else {
            panic!("`{}` was not drawn", note.name);
        };
        assert!(walked >= 2, "`{}` runs through {walked} places", note.name);
        assert!(stray <= 0.01, "`{}` strays {stray} cm", note.name);
        places += walked;
        curves += bends;
        if on_contour > 0 {
            anchored.push((note.name.as_str(), on_contour));
        }
    }
    assert_eq!((places, curves), (71, 13));
    // `quiebre` runs from the waist to the hem down the middle of the front,
    // and the hem end of it is a corner of that very piece.
    assert_eq!(anchored, [("quiebre", 1)]);
    let on_contour: usize = product
        .doc
        .lines
        .iter()
        .flat_map(|(_, line)| line.vertices())
        .filter(|place| place.anchor().is_some())
        .count();
    assert_eq!(on_contour, 1);
    for (_, line) in product.doc.lines.iter() {
        for place in line.vertices() {
            if let LineVertex::Contour(anchor) = place {
                assert_eq!(anchor.piece, line.piece);
                let piece = product.doc.pieces.get(line.piece).expect("live");
                assert!(piece.node_index(anchor.from).is_some());
            }
        }
    }
}

/// Every place of every line lands where the Seamly path's own evaluator puts
/// it, on the owner's body and on a body grown in the hip and the inseam.
///
/// The walk is over the lines and not over the whole arena, so a place that
/// stops following its path fails here by name instead of disappearing into a
/// total. On the grown body the places that hang on nothing frozen are held to
/// the same tolerance; the rest are held to the drift the report accounts for.
#[test]
fn every_place_of_every_line_lands_where_the_pattern_puts_it_on_either_body() {
    let mut product = product();
    let mut counted = [0_usize; 2];
    let mut worst = [0.0_f64; 2];
    for grown in [false, true] {
        let body = if grown { grow(&mut product) } else { body() };
        let evaluated = reference(&body);
        let resolved = resolve(&product.doc);
        for (_, line) in product.doc.lines.iter() {
            let places = line.vertices().filter_map(LineVertex::point);
            for key in places.chain(line.handles()) {
                let source = product.sources[&key];
                let want = source.locate(&evaluated).expect("the path evaluates");
                let gap = distance(resolved[&key], want);
                // On the imported body nothing has drifted yet: every frozen
                // quantity was frozen at it.
                let follows = !grown || !product.frozen.contains_key(&key);
                let bound = if follows { 1e-6 } else { 3.5 };
                assert!(gap <= bound, "{source:?} lands {gap:e} cm off");
                counted[usize::from(!follows)] += 1;
                worst[usize::from(!follows)] = worst[usize::from(!follows)].max(gap);
            }
        }
    }
    assert_eq!(counted, [157, 35]);
    println!(
        "lines: {} places follow within {:e} cm, {} hang on something frozen, worst {:.4} cm",
        counted[0], worst[0], counted[1], worst[1]
    );
}

/// The same body grown in the hip, in the product and in the pattern alike.
fn grow(product: &mut toile_seamly::Product) -> toile_seamly::Measurements {
    use toile_doc::Command;
    use toile_seamly::seamly_measurement;

    let mut seamly = body();
    let mannequin = product.doc.resolve_with;
    for (toile, by) in [("cadera", 4.0), ("entrepierna", 3.0)] {
        let name = seamly_measurement(toile).expect("the mapping names both");
        let value = seamly.get(name).expect("the owner's body carries both") + by;
        seamly = seamly.with_value(name, value).expect("the body has it");
        Command::SetMeasure {
            mannequin,
            name: toile.to_owned(),
            to: value,
        }
        .apply(&mut product.doc)
        .expect("the body carries the measure");
    }
    seamly
}
