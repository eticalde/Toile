use toile_engine::draft::{Binding, Doc, MeasureSet, Point, PointKey};

use super::{beyond, written_alike};

/// A document holding three points: two written with one pair of expressions,
/// and one written with a different pair that is worth the same numbers.
fn points() -> (Doc, [PointKey; 3]) {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    let bound = |source: &str| Binding::parse(source).expect("the grammar of the pattern");
    let keys = [
        doc.points.insert(Point::at(bound("cintura / 4"), 0.0)),
        doc.points.insert(Point::at(bound("cintura / 4"), 0.0)),
        doc.points.insert(Point::at(bound("cintura / 8 * 2"), 0.0)),
    ];
    (doc, keys)
}

/// Two nodes are one place when the document spells them the same way, and two
/// spellings worth the same numbers are two places.
///
/// The bindings and not the numbers, because that is the comparison the
/// document itself makes when it refuses a flat wedge: the same document is the
/// same document whichever body it is read against, and only the text is.
#[test]
fn nodes_are_one_place_when_the_document_spells_them_one_way() {
    let (doc, keys) = points();
    assert!(written_alike(&doc, keys[0], keys[1]));
    assert!(
        !written_alike(&doc, keys[0], keys[2]),
        "two spellings are two places, whatever they are worth today"
    );
    let gone = PointKey::new(9, 0);
    assert!(
        !written_alike(&doc, keys[0], gone),
        "a key the document does not hold is not written anywhere"
    );
}

/// The walk steps on in the direction it came from, and off the head of the
/// contour it does not step at all.
///
/// That last one is what keeps a wedge off the place where the contour closes:
/// the document counts a wedge's seats forward from its first leg and never
/// wraps them, so a wedge that straddles the closure is one it will not hold.
#[test]
fn the_walk_steps_on_in_the_direction_it_came_from() {
    assert_eq!(beyond(1, 2), Some(3), "forward along the contour");
    assert_eq!(beyond(3, 2), Some(1), "and backward along it");
    assert_eq!(beyond(1, 0), None, "off the head, so nowhere");
}
