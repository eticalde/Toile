#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_doc::{
    Command, Doc, DocError, EdgeRange, Identity, PieceKey, PointKey, Seam, SeamOrientation, block,
};

fn doc() -> Doc {
    block::trousers()
}

fn piece(doc: &Doc, name: &str) -> PieceKey {
    doc.piece_named(name).expect("the block draws it")
}

fn node(doc: &Doc, piece: PieceKey, label: &str) -> PointKey {
    doc.shows_label(piece, label)
        .unwrap_or_else(|| panic!("the block names {label}"))
}

/// A third seam a test can add: the two hems, sewn to each other.
fn hem_to_hem(doc: &Doc) -> Seam {
    let front = piece(doc, block::FRONT);
    let back = piece(doc, block::BACK);
    Seam::plain(
        EdgeRange::between(
            front,
            node(doc, front, "bajo_lat"),
            node(doc, front, "bajo_int"),
        ),
        EdgeRange::between(
            back,
            node(doc, back, "bajo_lat_tras"),
            node(doc, back, "bajo_int_tras"),
        ),
        SeamOrientation::Aligned,
    )
}

#[test]
fn adding_and_unpicking_a_seam_restores_the_same_key() {
    let mut doc = doc();
    let added = Command::AddSeam {
        identity: Identity::New,
        seam: hem_to_hem(&doc),
    }
    .apply(&mut doc)
    .expect("every anchor is a live node");
    let Command::RemoveSeam { seam: key } = added.inverse.clone() else {
        panic!("the inverse of a sewing is an unpicking");
    };
    let held = doc.seams.get(key).copied().expect("the seam landed");

    let removed = added.inverse.apply(&mut doc).expect("the seam is live");
    assert!(doc.seams.get(key).is_none());
    removed.inverse.apply(&mut doc).expect("the slot is free");
    assert_eq!(doc.seams.get(key), Some(&held), "the same key lives again");
}

#[test]
fn a_sewing_names_the_two_pieces_it_touches() {
    let mut doc = doc();
    let front = piece(&doc, block::FRONT);
    let back = piece(&doc, block::BACK);
    let applied = Command::AddSeam {
        identity: Identity::New,
        seam: hem_to_hem(&doc),
    }
    .apply(&mut doc)
    .expect("every anchor is a live node");
    assert_eq!(applied.touched, [front, back]);
}

#[test]
fn a_seam_citing_a_missing_point_is_refused_by_name() {
    let mut doc = doc();
    let mut seam = hem_to_hem(&doc);
    seam.b.tail.from = PointKey::new(90, 0);
    let refused = Command::AddSeam {
        identity: Identity::New,
        seam,
    }
    .apply(&mut doc)
    .expect_err("the point does not exist");
    assert_eq!(refused.to_string(), "`Point` has no entry 90.0");
    assert_eq!(doc.seams.len(), 2, "nothing landed");
}

#[test]
fn a_seam_citing_a_missing_piece_is_refused_by_name() {
    let mut doc = doc();
    let mut seam = hem_to_hem(&doc);
    let stray = PieceKey::new(7, 0);
    seam.a.head.piece = stray;
    seam.a.tail.piece = stray;
    let refused = Command::AddSeam {
        identity: Identity::New,
        seam,
    }
    .apply(&mut doc)
    .expect_err("the piece does not exist");
    assert_eq!(refused.to_string(), "`Piece` has no entry 7.0");
}

#[test]
fn a_seam_anchored_on_a_handle_is_refused_as_no_node() {
    let mut doc = doc();
    let front = piece(&doc, block::FRONT);
    let handle = doc
        .pieces
        .get(front)
        .and_then(|held| held.contour.iter().find_map(|n| n.segment.handles()))
        .map(|(out, _)| out)
        .expect("the front bends two tracts");
    let mut seam = hem_to_hem(&doc);
    seam.a.head.from = handle;
    let refused = Command::AddSeam {
        identity: Identity::New,
        seam,
    }
    .apply(&mut doc)
    .expect_err("nothing sews to a handle");
    assert_eq!(refused, DocError::NoSuchNode);
}

#[test]
fn a_seam_side_split_across_two_pieces_is_refused() {
    let mut doc = doc();
    let mut seam = hem_to_hem(&doc);
    seam.a.tail.piece = piece(&doc, block::BACK);
    let refused = Command::AddSeam {
        identity: Identity::New,
        seam,
    }
    .apply(&mut doc)
    .expect_err("a side cannot straddle two pieces");
    assert_eq!(refused, DocError::SplitSeamSide);
}

#[test]
fn unpicking_a_seam_that_is_not_there_is_an_error_not_a_panic() {
    let mut doc = doc();
    let refused = Command::RemoveSeam {
        seam: toile_doc::SeamKey::new(9, 0),
    }
    .apply(&mut doc)
    .expect_err("the key names nothing");
    assert_eq!(refused.to_string(), "`Seam` has no entry 9.0");
}
