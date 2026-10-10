use toile_engine::draft::{DocError, EdgeAnchor, MannequinKey, block};

use super::*;

/// The waist of the shipped block, which is a run of one piece.
fn waist(doc: &Doc) -> EdgeRange {
    let piece = doc.piece_named(block::FRONT).expect("the block draws one");
    let named = |label: &str| {
        doc.shows_label(piece, label)
            .unwrap_or_else(|| panic!("the block names {label}"))
    };
    EdgeRange::between(piece, named("cintura_cf"), named("cintura_lat"))
}

/// Stepping the ring reaches every station the document takes, and only
/// those.
///
/// A ring that missed a girth would leave a station nothing in the app can
/// ask for; one that offered a length would ask for a station the document
/// refuses, and the refusal would reach the person in English.
#[test]
fn the_step_reaches_every_station_the_document_takes_and_closes_the_ring() {
    let mut seen = vec![Hang::WAIST];
    let mut station = Hang::WAIST;
    for _ in 0..MeasureSet::GIRTHS.len() {
        station = next(station).expect("a girth steps to a girth");
        assert!(Hang::names_a_ring(station), "{station}");
        if station == Hang::WAIST {
            break;
        }
        assert!(!seen.contains(&station), "{station} twice round");
        seen.push(station);
    }
    assert_eq!(station, Hang::WAIST, "the ring closes");
    assert_eq!(seen.len(), MeasureSet::GIRTHS.len(), "{seen:?}");
    for name in MeasureSet::LENGTHS.iter().chain(&MeasureSet::WHOLE) {
        assert_eq!(next(name), None, "{name}");
    }
    assert_eq!(next("largo_manga"), None);
}

/// A run whose ends are on two pieces is refused in Spanish, and not by
/// repeating what the document would have said.
#[test]
fn a_run_across_two_pieces_is_refused_in_spanish() {
    let doc = block::trouser_front();
    let run = waist(&doc);
    assert_eq!(refused(&doc, run), None, "one piece, both ends");
    let split = EdgeRange {
        tail: EdgeAnchor {
            piece: PieceKey::new(9, 0),
            ..run.tail
        },
        ..run
    };
    let why = refused(&doc, split).expect("a split run cannot be hung");
    assert_eq!(why, SPLIT);
    assert_ne!(
        why,
        DocError::SplitHang.to_string(),
        "in Spanish, and whole"
    );
}

/// And so is a product that resolves against no body.
#[test]
fn a_product_that_resolves_against_no_body_is_refused_in_spanish() {
    let mut doc = block::trouser_front();
    doc.resolve_with = MannequinKey::new(9, 0);
    assert_eq!(doc.measures(), None, "the key names no body");
    assert_eq!(refused(&doc, waist(&doc)), Some(NO_BODY));
}
