#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_doc::{
    ChangeClass, Coalesced, Command, Doc, DocError, EdgeAnchor, EdgeRange, Hang, HangKey, History,
    Identity, MeasureSet, PieceKey, PointKey, block,
};

const DRAG: &str = "colgar la prenda";

fn front(doc: &Doc) -> PieceKey {
    doc.piece_named(block::FRONT).expect("the block draws one")
}

fn node(doc: &Doc, label: &str) -> PointKey {
    doc.shows_label(front(doc), label)
        .unwrap_or_else(|| panic!("the block names {label}"))
}

/// The front's waistline, hung from the body's own waist.
fn waistband(doc: &Doc) -> Hang {
    let run = EdgeRange::between(
        front(doc),
        node(doc, "cintura_cf"),
        node(doc, "cintura_lat"),
    );
    Hang::new(run, Hang::WAIST)
}

fn add(doc: &mut Doc, hang: Hang) -> HangKey {
    let applied = Command::AddHang {
        identity: Identity::New,
        hang,
    }
    .apply(doc)
    .expect("both ends are nodes of the front");
    match applied.inverse {
        Command::RemoveHang { hang } => hang,
        other => panic!("the inverse of hanging a stretch is letting it off: {other:?}"),
    }
}

fn hung(doc: &Doc, key: HangKey) -> Hang {
    doc.hangs.get(key).expect("the hang is live").clone()
}

/// Nothing is drawn differently and no node is gained, and still the cloth of
/// the piece has to be read again: the run of vertices a hang holds is read off
/// the resolved contour, so it is priced with the edits that re-derive it.
#[test]
fn hanging_a_stretch_from_the_body_is_priced_as_a_change_of_shape() {
    let mut doc = block::trouser_front();
    let command = Command::AddHang {
        identity: Identity::New,
        hang: waistband(&doc),
    };
    assert_eq!(command.class(), ChangeClass::Shape);
    let applied = command.apply(&mut doc).expect("both ends are nodes");
    assert_eq!(applied.class, ChangeClass::Shape);
    assert_eq!(applied.touched, [front(&doc)]);
    assert_eq!(doc.hangs.len(), 1);
}

#[test]
fn hanging_a_stretch_and_letting_it_off_restores_the_same_key() {
    let mut doc = block::trouser_front();
    let band = waistband(&doc);
    let key = add(&mut doc, band);
    let before = doc.clone();
    let applied = Command::RemoveHang { hang: key }
        .apply(&mut doc)
        .expect("the hang is live");
    assert!(doc.hangs.is_empty());
    applied
        .inverse
        .apply(&mut doc)
        .expect("the slot is free again");
    assert_eq!(doc, before, "the same key, the same station, the same run");
    assert_eq!(hung(&doc, key).station, "cintura");
}

/// The station is a whole field, so a person turning a chooser through four
/// rings inside one gesture leaves one entry, as dragging a slider does.
#[test]
fn turning_the_station_through_a_gesture_leaves_one_entry() {
    let mut doc = block::trouser_front();
    let band = waistband(&doc);
    let key = add(&mut doc, band);
    let mut history = History::new();
    history.begin(DRAG);
    for station in ["cadera", "muslo", "rodilla"] {
        history
            .edit(
                &mut doc,
                Command::SetHangStation {
                    hang: key,
                    to: station.to_owned(),
                },
            )
            .expect("the hang is live");
    }
    history.end();
    assert_eq!(hung(&doc, key).station, "rodilla");
    history.undo(&mut doc).expect("one entry for the gesture");
    assert_eq!(hung(&doc, key).station, "cintura", "back to where it hung");

    let turn = Command::SetHangStation {
        hang: key,
        to: "cadera".to_owned(),
    };
    assert_eq!(turn.coalesce_onto(&turn), Coalesced::Replaces);
    let put_on = Command::AddHang {
        identity: Identity::New,
        hang: waistband(&doc),
    };
    assert_eq!(
        put_on.coalesce_onto(&put_on),
        Coalesced::Separate,
        "two hangs are two things done"
    );
}

/// A station the body carries no ring for is refused, and the hang keeps the
/// one it had: a chooser that offered a name nothing answers to would leave the
/// garment hanging from nothing.
#[test]
fn a_station_with_no_ring_behind_it_is_refused_and_changes_nothing() {
    let mut doc = block::trouser_front();
    let band = waistband(&doc);
    let key = add(&mut doc, band);
    for station in ["estatura", "tiro", "waist", ""] {
        assert_eq!(
            Command::SetHangStation {
                hang: key,
                to: station.to_owned(),
            }
            .apply(&mut doc),
            Err(DocError::HangStation(station.to_owned())),
            "{station}"
        );
        assert_eq!(hung(&doc, key).station, "cintura", "{station}");
    }
    for station in MeasureSet::GIRTHS {
        Command::SetHangStation {
            hang: key,
            to: station.to_owned(),
        }
        .apply(&mut doc)
        .unwrap_or_else(|why| panic!("{station}: {why}"));
        assert_eq!(hung(&doc, key).station, station);
    }
}

/// A run whose two ends sit on different pieces names no cloth, and a fraction
/// no tract can answer for is refused at the one door every stretch comes
/// through.
#[test]
fn a_run_no_piece_answers_for_is_refused() {
    let mut doc = block::trousers();
    let run = waistband(&doc).at;
    let split = EdgeRange {
        tail: EdgeAnchor {
            piece: PieceKey::new(1, 0),
            ..run.tail
        },
        ..run
    };
    assert_eq!(
        Command::AddHang {
            identity: Identity::New,
            hang: Hang::new(split, Hang::WAIST),
        }
        .apply(&mut doc),
        Err(DocError::SplitHang)
    );
    let off_the_tract = EdgeRange {
        head: EdgeAnchor { t: 1.5, ..run.head },
        ..run
    };
    assert_eq!(
        Command::AddHang {
            identity: Identity::New,
            hang: Hang::new(off_the_tract, Hang::WAIST),
        }
        .apply(&mut doc),
        Err(DocError::AnchorFraction)
    );
    assert!(doc.hangs.is_empty());
}

/// A hang names a piece and its nodes and nothing names a hang back, so a
/// product with none of them is the document it was, and a key that names no
/// live entry is reported rather than panicked on.
#[test]
fn a_product_hangs_from_nothing_until_somebody_hangs_it() {
    let mut doc = block::trouser_front();
    assert!(doc.hangs.is_empty());
    let stale = HangKey::new(3, 0);
    assert_eq!(
        Command::RemoveHang { hang: stale }.apply(&mut doc),
        Err(DocError::stale(stale))
    );
    assert_eq!(
        Command::SetHangStation {
            hang: stale,
            to: Hang::WAIST.to_owned(),
        }
        .apply(&mut doc),
        Err(DocError::stale(stale))
    );
}
