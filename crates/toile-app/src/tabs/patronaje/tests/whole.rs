use eframe::egui::{Event, vec2};
use toile_engine::draft::{Draft, PieceKey, block};

use super::super::state::Scope;
use super::super::{State, layout};
use super::bench::{Bench, front_and_back};
use super::button;

/// Opening a product lays its pieces out apart, and looking at them writes
/// nothing: the row is the overview's, not the file's.
#[test]
fn a_product_opens_on_all_its_pieces_and_looking_at_them_writes_nothing() {
    let shipped = block::trousers();
    let mut bench = Bench::new(shipped.clone());
    bench.frame(Vec::new());
    let (front, back) = front_and_back(bench.doc());
    for piece in [back, front] {
        let at = bench.on_product(piece);
        bench.frame(vec![Event::PointerMoved(at)]);
    }
    assert_eq!(bench.state.scope, Scope::Product);
    assert_eq!(bench.session.revision(), 0, "no edit was played");
    assert!(!bench.session.can_undo());
    assert_eq!(bench.doc().to_canonical_json(), shipped.to_canonical_json());
    let laid = layout::of(bench.session.draft().expect("a product is open"));
    let [a, b] = [&laid[0], &laid[1]]
        .map(|it| layout::bounds(std::slice::from_ref(it)).expect("an outline"));
    assert!(a.right() < b.left(), "side by side, apart: {a:?} {b:?}");
}

#[test]
fn dragging_a_piece_is_one_entry_that_moves_where_it_sits_and_nothing_else() {
    let shipped = block::trousers();
    let mut bench = Bench::new(shipped.clone());
    bench.frame(Vec::new());
    let (front, back) = front_and_back(bench.doc());
    let resolved = |draft: &Draft| {
        [front, back].map(|piece| {
            (
                draft.points_cm(piece).to_vec(),
                draft.flat_cm(piece).to_vec(),
            )
        })
    };
    let draft = bench.session.draft().expect("a product is open");
    let before = resolved(draft);
    let laid = layout::of(draft);
    let from = laid.iter().find(|it| it.piece == back).expect("laid").shift;

    let grab = bench.on_product(back);
    let run = vec2(90.0, 36.0);
    bench.frame(vec![Event::PointerMoved(grab)]);
    bench.frame(vec![button(grab, true)]);
    for step in 1..=3 {
        bench.frame(vec![Event::PointerMoved(grab + run * step as f32 / 3.0)]);
    }
    bench.frame(vec![button(grab + run, false)]);

    let doc = bench.doc();
    let placement = |piece| doc.pieces.get(piece).expect("the key is live").placement;
    assert_eq!(placement(front), None, "the other piece is not arranged");
    let placed = placement(back).expect("the dragged piece is placed");
    let scale = bench.state.view.scale();
    let want = [
        from[0] + f64::from(run.x) / scale,
        from[1] + f64::from(run.y) / scale,
    ];
    assert!(
        (placed.x - want[0]).abs() < 0.051 && (placed.y - want[1]).abs() < 0.051,
        "{placed:?} against {want:?}"
    );
    let mut unplaced = doc.clone();
    unplaced.pieces.get_mut(back).expect("live").placement = None;
    assert_eq!(
        unplaced, shipped,
        "nothing but where the piece sits changed"
    );
    let draft = bench.session.draft().expect("a product is open");
    assert_eq!(resolved(draft), before, "no resolved coordinate moved");
    assert_eq!(bench.session.undo_label(), Some("mover pieza"));
    assert_eq!(
        bench.state.active,
        Some(back),
        "the press chose what it moved"
    );
    assert_eq!(bench.state.scope, Scope::Product);

    bench.session.undo().expect("the entry is taken back");
    assert_eq!(bench.doc().pieces.get(back).expect("live").placement, None);
    assert!(!bench.session.can_undo(), "the whole drag was one entry");
}

#[test]
fn a_double_click_on_a_piece_opens_it_alone() {
    let mut bench = Bench::new(block::trousers());
    bench.frame(Vec::new());
    let (_, back) = front_and_back(bench.doc());
    let at = bench.on_product(back);
    bench.click(at);
    assert_eq!(bench.state.scope, Scope::Product, "one click only chooses");
    assert_eq!(bench.state.active, Some(back));
    bench.frame(vec![button(at, true)]);
    bench.frame(vec![button(at, false)]);
    assert_eq!(bench.state.scope, Scope::Piece);
    assert_eq!(bench.state.active, Some(back));
    assert_eq!(bench.session.revision(), 0, "opening a piece edits nothing");
}

#[test]
fn every_product_put_on_the_table_opens_on_all_its_pieces() {
    let mut state = State::default();
    assert_eq!(
        state.scope,
        Scope::Product,
        "the tab starts on the whole product"
    );
    state.open(PieceKey::new(1, 0));
    state.reset();
    assert_eq!(
        state.scope,
        Scope::Product,
        "and so does every product after it"
    );
}
