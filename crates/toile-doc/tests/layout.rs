#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_doc::{
    ChangeClass, Coalesced, Command, Doc, DocError, FORMAT_VERSION, FORMAT_VERSION_PLACED, Grain,
    History, Identity, PieceKey, Placement, block,
};

const DRAG: &str = "mover pieza";

/// The front and the back of the shipped block.
fn pieces(doc: &Doc) -> (PieceKey, PieceKey) {
    let front = doc.piece_named(block::FRONT).expect("the block draws one");
    let back = doc.piece_named(block::BACK).expect("the block draws one");
    (front, back)
}

fn place(piece: PieceKey, x: f64, y: f64) -> Command {
    Command::PlacePiece {
        piece,
        to: Some(Placement::new(x, y)),
    }
}

fn placement(doc: &Doc, piece: PieceKey) -> Option<Placement> {
    doc.pieces.get(piece).expect("the key is live").placement
}

/// One drag of `piece` across the overview, frame by frame.
fn drag(history: &mut History, doc: &mut Doc, piece: PieceKey, frames: &[(f64, f64)]) {
    history.begin(DRAG);
    for &(x, y) in frames {
        history
            .edit(doc, place(piece, x, y))
            .expect("the piece is live");
    }
    history.end();
}

/// Where a piece sits on the overview is layout: nothing resolves, meshes or
/// drapes from it. A piece named as touched would be re-derived, and re-draped,
/// when the history steps over the edit, so none is named.
#[test]
fn placing_a_piece_costs_the_derivation_nothing() {
    let original = block::trousers();
    let mut doc = original.clone();
    let (front, _) = pieces(&doc);
    let command = place(front, 42.5, -3.0);
    assert_eq!(command.class(), ChangeClass::Metadata);
    let applied = command.apply(&mut doc).expect("the piece is live");
    assert_eq!(applied.class, ChangeClass::Metadata);
    assert!(applied.touched.is_empty());
    assert_eq!(
        applied.inverse,
        Command::PlacePiece {
            piece: front,
            to: None
        }
    );

    let mut unplaced = doc.clone();
    unplaced
        .pieces
        .get_mut(front)
        .expect("the key is live")
        .placement = None;
    assert_eq!(unplaced, original);
}

#[test]
fn a_placement_set_and_undone_takes_the_file_to_version_4_and_back() {
    let mut doc = block::trousers();
    let before = doc.to_canonical_json();
    let (front, _) = pieces(&doc);
    let mut history = History::new();
    history
        .edit(&mut doc, place(front, 42.5, -3.0))
        .expect("the piece is live");
    assert_eq!(doc.format_version(), FORMAT_VERSION_PLACED);
    let file = doc.to_canonical_json();
    assert!(
        file.contains("\"placement\": {\n            \"x\": 42.5,\n            \"y\": -3\n"),
        "{file}"
    );
    assert_eq!(Doc::from_json(&file), Ok(doc.clone()));

    assert_eq!(history.undo(&mut doc), Ok(Vec::new()));
    assert_eq!(doc.format_version(), FORMAT_VERSION);
    assert_eq!(doc.to_canonical_json(), before);
    assert_eq!(history.redo(&mut doc), Ok(Vec::new()));
    assert_eq!(doc.to_canonical_json(), file);
}

/// A drag emits a placement per frame, and what the history keeps is the same
/// as a single jump to where the drag let go.
#[test]
fn one_drag_of_a_piece_folds_into_one_edit_that_undoes_to_not_arranged() {
    let mut doc = block::trousers();
    let (front, _) = pieces(&doc);
    let mut jumped_doc = doc.clone();
    let mut jumped = History::new();
    drag(&mut jumped, &mut jumped_doc, front, &[(12.0, 4.0)]);

    let mut history = History::new();
    let frames = [(1.0, 0.5), (6.5, 2.0), (12.0, 4.0)];
    drag(&mut history, &mut doc, front, &frames);
    assert_eq!(history, jumped);
    assert_eq!(doc, jumped_doc);

    history.undo(&mut doc).expect("the piece is live");
    assert_eq!(placement(&doc, front), None);
    history.redo(&mut doc).expect("the piece is live");
    assert_eq!(placement(&doc, front), Some(Placement::new(12.0, 4.0)));
}

#[test]
fn a_placement_folds_onto_its_own_piece_and_nothing_else() {
    let doc = block::trousers();
    let (front, back) = pieces(&doc);
    let clear = Command::PlacePiece {
        piece: front,
        to: None,
    };
    let grain = Command::SetGrain {
        piece: front,
        to: Grain::Angle(0.0),
    };
    let earlier = place(front, 1.0, 1.0);
    assert_eq!(
        place(front, 2.0, 2.0).coalesce_onto(&earlier),
        Coalesced::Replaces
    );
    assert_eq!(clear.coalesce_onto(&earlier), Coalesced::Replaces);
    assert_eq!(
        place(back, 1.0, 1.0).coalesce_onto(&earlier),
        Coalesced::Separate
    );
    assert_eq!(earlier.coalesce_onto(&grain), Coalesced::Separate);
}

/// Two pieces dragged together stay two edits: folded into one, the undo
/// would leave one of them arranged.
#[test]
fn two_pieces_dragged_in_one_gesture_both_undo_to_not_arranged() {
    let mut doc = block::trousers();
    let before = doc.clone();
    let (front, back) = pieces(&doc);
    let mut history = History::new();
    history.begin(DRAG);
    for frame in [1.0, 2.0] {
        history
            .edit(&mut doc, place(front, frame, 0.0))
            .expect("the piece is live");
        history
            .edit(&mut doc, place(back, 0.0, frame))
            .expect("the piece is live");
    }
    history.end();
    history.undo(&mut doc).expect("both pieces are live");
    assert_eq!(doc, before);
}

/// The history is opaque, and a fold past the addition would change no state
/// an undo reaches today, so the entry is read through its debug text: two
/// placements and their two inverses, neither frame standing for the other.
#[test]
fn a_drag_does_not_fold_back_past_an_edit_that_makes_a_piece() {
    let mut doc = block::trousers();
    let (front, back) = pieces(&doc);
    let mut pocket = doc.pieces.get(back).expect("the key is live").clone();
    pocket.name = "Bolsillo".to_owned();
    let before = doc.clone();
    let mut history = History::new();
    history.begin(DRAG);
    let add = Command::AddPiece {
        identity: Identity::New,
        piece: pocket,
    };
    for command in [place(front, 1.0, 0.0), add, place(front, 2.0, 0.0)] {
        history.edit(&mut doc, command).expect("each step is legal");
    }
    history.end();
    let recorded = format!("{history:?}");
    assert_eq!(recorded.matches("PlacePiece").count(), 4, "{recorded}");

    let arranged = doc.clone();
    history.undo(&mut doc).expect("the entry undoes");
    assert_eq!(placement(&doc, front), None);
    assert_eq!(doc.pieces.len(), before.pieces.len());
    history.redo(&mut doc).expect("the entry redoes");
    assert_eq!(doc, arranged);
}

/// Sending a piece back to being laid out by the overview is an edit like any
/// other, and its undo puts the piece back where it had been put.
#[test]
fn clearing_a_placement_is_an_edit_its_undo_takes_back() {
    let mut doc = block::trousers();
    let (front, _) = pieces(&doc);
    let mut history = History::new();
    drag(&mut history, &mut doc, front, &[(5.0, 5.0)]);
    let clear = Command::PlacePiece {
        piece: front,
        to: None,
    };
    let applied = history.edit(&mut doc, clear).expect("the piece is live");
    assert_eq!(applied.inverse, place(front, 5.0, 5.0));
    assert_eq!(placement(&doc, front), None);
    assert_eq!(doc.format_version(), FORMAT_VERSION);
    history.undo(&mut doc).expect("the piece is live");
    assert_eq!(placement(&doc, front), Some(Placement::new(5.0, 5.0)));
}

/// JSON has no NaN or infinity: autosaved, the placement would be written as
/// `null` and the product would never reopen.
#[test]
fn a_placement_the_file_cannot_spell_is_refused_before_it_is_written() {
    let mut doc = block::trousers();
    let before = doc.to_canonical_json();
    let (front, _) = pieces(&doc);
    let mut history = History::new();
    for (x, y) in [
        (f64::NAN, 0.0),
        (0.0, f64::INFINITY),
        (f64::NEG_INFINITY, 1.0),
    ] {
        assert_eq!(
            history.edit(&mut doc, place(front, x, y)),
            Err(DocError::NonFinite("placement".to_owned()))
        );
    }
    assert_eq!(history.depth(), 0);
    assert_eq!(doc.to_canonical_json(), before);
    let stale = place(PieceKey::new(9, 0), 0.0, 0.0);
    assert!(matches!(
        stale.apply(&mut doc),
        Err(DocError::StaleKey { .. })
    ));
}
