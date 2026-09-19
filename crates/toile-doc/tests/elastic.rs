#![allow(missing_docs, reason = "a test crate publishes no API surface")]
#![allow(
    clippy::float_cmp,
    reason = "an elastic reads back the very ratio it was given"
)]

use toile_doc::{
    ChangeClass, Coalesced, Command, Doc, DocError, EdgeRange, Elastic, ElasticKey, History,
    Identity, PieceKey, Point, PointKey, SegmentEdit, block,
};

const DRAG: &str = "tensar elástico";

fn front(doc: &Doc) -> PieceKey {
    doc.piece_named(block::FRONT).expect("the block draws one")
}

fn node(doc: &Doc, label: &str) -> PointKey {
    doc.shows_label(front(doc), label)
        .unwrap_or_else(|| panic!("the block names {label}"))
}

/// An elastic across the waist of the front, at 85 % of its drawn length.
fn waistband(doc: &Doc) -> Elastic {
    let stretch = EdgeRange::between(front(doc), node(doc, "cintura_cf"), node(doc, "cadera_lat"));
    Elastic::new(stretch, 0.85, 30.0)
}

fn add(doc: &mut Doc, elastic: Elastic) -> ElasticKey {
    let applied = Command::AddElastic {
        identity: Identity::New,
        elastic,
    }
    .apply(doc)
    .expect("both ends are nodes of the front");
    match applied.inverse {
        Command::RemoveElastic { elastic } => elastic,
        other => panic!("the inverse of holding a stretch in is letting it out: {other:?}"),
    }
}

/// The waistband, put on the front, named by its key.
fn banded(doc: &mut Doc) -> ElasticKey {
    let elastic = waistband(doc);
    add(doc, elastic)
}

fn held(doc: &Doc, key: ElasticKey) -> Elastic {
    *doc.elastics.get(key).expect("the elastic is live")
}

/// Nothing is drawn differently and no node is gained, and still the cloth of
/// the piece changes: the class is the one whose budget is the rest lengths.
#[test]
fn holding_a_stretch_in_costs_the_piece_its_rest_lengths() {
    let mut doc = block::trouser_front();
    let command = Command::AddElastic {
        identity: Identity::New,
        elastic: waistband(&doc),
    };
    assert_eq!(command.class(), ChangeClass::Shape);
    let applied = command.apply(&mut doc).expect("both ends are nodes");
    assert_eq!(applied.class, ChangeClass::Shape);
    assert_eq!(applied.touched, [front(&doc)]);
    assert_eq!(doc.elastics.len(), 1);
}

#[test]
fn putting_an_elastic_on_and_taking_it_off_restores_the_same_key() {
    let mut doc = block::trouser_front();
    let key = banded(&mut doc);
    let elastic = held(&doc, key);

    let removed = Command::RemoveElastic { elastic: key }
        .apply(&mut doc)
        .expect("the elastic is live");
    assert!(doc.elastics.get(key).is_none());
    removed.inverse.apply(&mut doc).expect("the slot is free");
    assert_eq!(doc.elastics.get(key), Some(&elastic), "the same key lives");
}

/// JSON has no NaN or infinity, and 200 % is the most a stretch stands in for.
/// Autosaved, either would be written `null` and the product would not reopen.
#[test]
fn a_ratio_no_elastic_holds_is_refused_before_it_is_written() {
    let mut doc = block::trouser_front();
    let key = banded(&mut doc);
    let stretch = waistband(&doc).at;
    let mut history = History::new();
    for ratio in [0.0, -1.0, 2.5, f64::NAN, f64::INFINITY] {
        let slider = Command::SetElasticRatio {
            elastic: key,
            to: ratio,
        };
        assert_eq!(history.edit(&mut doc, slider), Err(DocError::ElasticRatio));
        let fresh = Command::AddElastic {
            identity: Identity::New,
            elastic: Elastic::new(stretch, ratio, 30.0),
        };
        assert_eq!(fresh.apply(&mut doc), Err(DocError::ElasticRatio));
    }
    assert_eq!(history.depth(), 0);
    assert_eq!(held(&doc, key).ratio, 0.85);
    assert_eq!(doc.elastics.len(), 1, "nothing landed");
}

#[test]
fn a_strength_that_is_not_finite_or_is_under_the_floor_is_refused() {
    let mut doc = block::trouser_front();
    let key = banded(&mut doc);
    for to in [0.0, -30.0, 1.0e-300, f64::NAN, f64::INFINITY] {
        let slider = Command::SetElasticStrength { elastic: key, to };
        assert_eq!(slider.apply(&mut doc), Err(DocError::ElasticStrength));
    }
    assert_eq!(held(&doc, key).strength, 30.0);
}

/// A stretch answers to one rule whoever writes it, so an elastic is refused
/// exactly where a seam side is: on a handle, on a key that leads nowhere, and
/// across two pieces.
#[test]
fn an_elastic_is_anchored_by_the_rule_a_seam_side_answers_to() {
    let mut doc = block::trousers();
    let piece = front(&doc);
    let handle = doc
        .pieces
        .get(piece)
        .and_then(|held| held.contour.iter().find_map(|node| node.segment.handles()))
        .map(|(out, _)| out)
        .expect("the front bends two tracts");
    let stray = PointKey::new(90, 0);
    let mut on_handle = waistband(&doc);
    on_handle.at.head.from = handle;
    let mut on_nothing = waistband(&doc);
    on_nothing.at.tail.from = stray;
    let mut split = waistband(&doc);
    split.at.tail.piece = doc.piece_named(block::BACK).expect("the block draws one");

    for (elastic, expected) in [
        (on_handle, DocError::NoSuchNode),
        (on_nothing, DocError::stale(stray)),
        (split, DocError::SplitElastic),
    ] {
        let command = Command::AddElastic {
            identity: Identity::New,
            elastic,
        };
        assert_eq!(command.apply(&mut doc), Err(expected));
    }
    assert!(doc.elastics.is_empty(), "nothing landed");
}

/// The ratio is dragged by a slider, which emits an edit per frame, and what
/// the history keeps is the same as a single jump to where the drag let go.
#[test]
fn one_drag_of_the_ratio_folds_into_one_edit() {
    let mut doc = block::trouser_front();
    let key = banded(&mut doc);
    let ratio = |to| Command::SetElasticRatio { elastic: key, to };

    let mut jumped_doc = doc.clone();
    let mut jumped = History::new();
    jumped.begin(DRAG);
    jumped
        .edit(&mut jumped_doc, ratio(0.6))
        .expect("the elastic is live");
    jumped.end();

    let mut history = History::new();
    history.begin(DRAG);
    for frame in [0.84, 0.75, 0.6] {
        history
            .edit(&mut doc, ratio(frame))
            .expect("the elastic is live");
    }
    history.end();
    assert_eq!(history, jumped);
    assert_eq!(doc, jumped_doc);

    history.undo(&mut doc).expect("the elastic is live");
    assert_eq!(held(&doc, key).ratio, 0.85, "undo goes back to the gesture");
    history.redo(&mut doc).expect("the elastic is live");
    assert_eq!(held(&doc, key).ratio, 0.6);
}

#[test]
fn the_ratio_and_the_strength_of_one_elastic_are_two_fields() {
    let key = ElasticKey::new(0, 0);
    let ratio = |to| Command::SetElasticRatio { elastic: key, to };
    let strength = |to| Command::SetElasticStrength { elastic: key, to };
    assert_eq!(ratio(0.7).coalesce_onto(&ratio(0.8)), Coalesced::Replaces);
    assert_eq!(
        strength(20.0).coalesce_onto(&strength(30.0)),
        Coalesced::Replaces
    );
    assert_eq!(
        ratio(0.7).coalesce_onto(&strength(30.0)),
        Coalesced::Separate
    );
    let other = Command::SetElasticRatio {
        elastic: ElasticKey::new(1, 0),
        to: 0.7,
    };
    assert_eq!(other.coalesce_onto(&ratio(0.8)), Coalesced::Separate);
}

/// The ends are node keys with a fraction local to the tract leaving them, so
/// a node put inside the stretch is cloth the elastic now holds, and not a
/// reason for either end to move.
#[test]
fn a_node_inserted_inside_the_stretch_leaves_both_ends_where_they_were() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let key = banded(&mut doc);
    let before = held(&doc, key);
    let nodes = doc
        .pieces
        .get(piece)
        .expect("the key is live")
        .contour
        .len();

    Command::InsertNode {
        piece,
        after: Some(node(&doc, "cintura_lat")),
        identity: Identity::New,
        value: Point::at(30.0, 5.0),
        segment: SegmentEdit::Line,
        samples: 1,
    }
    .apply(&mut doc)
    .expect("the contour runs through the waist");

    let after = doc
        .pieces
        .get(piece)
        .expect("the key is live")
        .contour
        .len();
    assert_eq!(after, nodes + 1);
    assert_eq!(held(&doc, key), before);
}

/// The same as a seam: the key is kept rather than rewritten, the arena never
/// hands the slot to anything else, and the undo puts the node back under it.
#[test]
fn an_elastic_whose_node_is_deleted_survives_the_undo_cycle() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let key = banded(&mut doc);
    let before = held(&doc, key);
    let hip = node(&doc, "cadera_lat");
    let mut history = History::new();

    history
        .edit(&mut doc, Command::RemoveNode { piece, node: hip })
        .expect("the contour runs through the hip");
    assert_eq!(held(&doc, key).at.tail.from, hip);
    assert!(doc.points.get(hip).is_none());

    history.undo(&mut doc).expect("the slot is free again");
    assert_eq!(held(&doc, key), before);
    assert!(doc.points.get(hip).is_some());
}

#[test]
fn an_elastic_on_a_piece_taken_off_the_table_comes_back_with_it() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    banded(&mut doc);
    let before = doc.clone();
    let mut history = History::new();

    history
        .edit(&mut doc, Command::RemovePiece { piece })
        .expect("the piece is live");
    assert_eq!(doc.elastics.len(), 1, "the elastic waits for its piece");
    history.undo(&mut doc).expect("the slot is free again");
    assert_eq!(doc, before);
}

/// Two elastics over one stretch are kept, both of them. The document holds
/// keys and fractions and measures nothing, so what a stretch covers is a
/// question for whoever walks the resolved contour — and two seams over one
/// stretch are already allowed by the same document.
#[test]
fn two_elastics_over_one_stretch_are_both_kept() {
    let mut doc = block::trouser_front();
    let first = banded(&mut doc);
    let tighter = Elastic::new(waistband(&doc).at, 0.7, 12.0);
    let second = add(&mut doc, tighter);
    assert_ne!(first, second);
    assert_eq!(held(&doc, first).at, held(&doc, second).at);
    assert_eq!(doc.elastics.len(), 2);
}
