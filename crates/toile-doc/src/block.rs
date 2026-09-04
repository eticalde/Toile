use crate::{Binding, Doc, MeasureSet, Piece, Point, PointKey, Variable, Winding};

/// The measurements of the person Toile was written for, in centimetres.
const ETIENNE: [(&str, f64); 10] = [
    ("cintura", 84.0),
    ("cadera", 98.0),
    ("muslo", 58.0),
    ("rodilla", 40.0),
    ("tobillo", 24.0),
    ("tiro", 27.0),
    ("largo_lateral", 104.0),
    ("entrepierna", 78.0),
    ("altura_cadera", 20.0),
    ("estatura", 178.0),
];

/// A standard size, so that one pattern can be seen on two bodies.
const SIZE_42: [(&str, f64); 10] = [
    ("cintura", 88.0),
    ("cadera", 104.0),
    ("muslo", 62.0),
    ("rodilla", 42.0),
    ("tobillo", 25.0),
    ("tiro", 28.0),
    ("largo_lateral", 106.0),
    ("entrepierna", 79.0),
    ("altura_cadera", 21.0),
    ("estatura", 176.0),
];

/// The eases and the widths the draft is built on.
const VARIABLES: [(&str, &str); 5] = [
    ("holgura_cadera", "1"),
    ("ancho_rodilla", "24"),
    ("ancho_bajo", "22"),
    ("extension_tiro", "cadera / 16"),
    ("raya", "(cadera / 4 + holgura_cadera - extension_tiro) / 2"),
];

/// The nine nodes of the front, in contour order: name, then x and y.
const NODES: [(&str, &str, &str); 9] = [
    ("cintura_cf", "0", "0"),
    ("cintura_lat", "cintura / 4 + 1", "0"),
    ("cadera_lat", "cadera / 4 + holgura_cadera", "altura_cadera"),
    (
        "rodilla_lat",
        "raya + ancho_rodilla / 2",
        "(tiro + largo_lateral) / 2",
    ),
    ("bajo_lat", "raya + ancho_bajo / 2", "largo_lateral"),
    ("bajo_int", "raya - ancho_bajo / 2", "largo_lateral"),
    (
        "rodilla_int",
        "raya - ancho_rodilla / 2",
        "(tiro + largo_lateral) / 2",
    ),
    ("tiro_int", "-extension_tiro", "tiro"),
    ("tiro_cf", "0", "tiro - extension_tiro"),
];

/// The name the front carries in the product tree.
pub const FRONT: &str = "Delantero";

/// The trouser front Toile brings, drafted as formulas over generic
/// measurements.
///
/// One document, two bodies: the same nine nodes resolve against either, which
/// is the whole promise of the pattern in one function.
pub fn trouser_front() -> Doc {
    let mut doc = Doc::new(MeasureSet::new("Etienne", ETIENNE));
    doc.mannequins.insert(MeasureSet::new("Talla 42", SIZE_42));
    for (name, source) in VARIABLES {
        doc.variables.insert(Variable::new(name, binding(source)));
    }
    let points: Vec<PointKey> = NODES
        .iter()
        .map(|&(label, x, y)| {
            doc.points
                .insert(Point::at(binding(x), binding(y)).named(label))
        })
        .collect();
    doc.pieces
        .insert(Piece::polygon(FRONT, points, Winding::Cw));
    doc
}

/// The binding one of this file's own sources spells.
fn binding(source: &str) -> Binding {
    Binding::parse(source).expect("the block's own sources are written in this file and parse")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Grain, Segment};

    #[test]
    fn trouser_front_has_nine_nodes() {
        let doc = trouser_front();
        let piece = doc.piece_named(FRONT).and_then(|key| doc.pieces.get(key));
        let piece = piece.expect("the block draws one piece");
        assert_eq!(piece.contour.len(), 9);
        assert_eq!(doc.points.len(), 9);
        assert!(piece.contour.iter().all(|n| n.segment == Segment::Line));
    }

    #[test]
    fn the_front_declares_its_winding_and_its_grain() {
        let doc = trouser_front();
        let key = doc.piece_named(FRONT).expect("the block draws one piece");
        let piece = doc.pieces.get(key).expect("the key is live");
        assert_eq!(piece.winding, Winding::Cw);
        assert_eq!(piece.grain, Grain::VERTICAL);
    }

    #[test]
    fn the_block_offers_two_bodies_and_resolves_against_the_first() {
        let doc = trouser_front();
        assert_eq!(doc.mannequins.len(), 2);
        assert_eq!(doc.measures().map(|set| set.name.as_str()), Some("Etienne"));
        assert!(doc.mannequin_named("Talla 42").is_some());
    }

    #[test]
    fn every_body_carries_the_whole_catalogue() {
        let doc = trouser_front();
        for (_, set) in doc.mannequins.iter() {
            assert_eq!(set.values.len(), 10);
            assert!(set.uncatalogued().is_empty());
        }
    }

    #[test]
    fn the_block_adds_five_pattern_variables_to_the_two_tolerances() {
        let doc = trouser_front();
        assert_eq!(doc.variables.len(), 7);
        for (name, _) in VARIABLES {
            assert!(doc.variable_named(name).is_some(), "{name} is missing");
        }
    }

    #[test]
    fn every_node_carries_the_name_the_draft_calls_it_by() {
        let doc = trouser_front();
        let piece = doc.piece_named(FRONT).expect("the block draws one piece");
        for (label, _, _) in NODES {
            assert!(
                doc.shows_label(piece, label).is_some(),
                "{label} is missing"
            );
        }
        assert_eq!(doc.shows_label(piece, "P1"), None);
    }

    #[test]
    fn the_centre_front_is_bound_to_plain_numbers() {
        let doc = trouser_front();
        let piece = doc.piece_named(FRONT).expect("the block draws one piece");
        let waist = doc.shows_label(piece, "cintura_cf").expect("it is named");
        let point = doc.points.get(waist).expect("the key is live");
        assert_eq!(point.x, Binding::Literal(0.0));
        assert_eq!(point.y, Binding::Literal(0.0));
        assert!(!point.label_visible);
    }
}
