use toile_engine::draft::block;

use super::*;

/// The block on the table: the hip bends, the crotch bends, and the seven
/// other tracts are straight.
fn front() -> (Draft, PieceKey) {
    let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
    let piece = draft
        .doc()
        .piece_named(block::FRONT)
        .expect("the block draws one piece");
    (draft, piece)
}

#[test]
fn the_block_bends_twice_and_each_bend_names_two_handles() {
    let (draft, piece) = front();
    let bends = bends(&draft, piece);
    assert_eq!(bends.len(), 2);
    for bend in &bends {
        assert_ne!(bend.out.0, bend.into.0);
        assert_eq!(draft.resolved(bend.out.0), Some(bend.out.1));
    }
}

#[test]
fn a_handle_names_the_node_it_hangs_from() {
    let (draft, piece) = front();
    let doc = draft.doc();
    let bend = bends(&draft, piece)[0];
    let leaving = hangs(doc, piece, bend.out.0).expect("the handle is on the piece");
    assert_eq!(leaving.node, bend.node);
    assert_eq!(leaving.side, Side::Out);
    // Nothing bends into the waist, so the tangent there has no mate.
    assert_eq!(leaving.mate, None);
    let arriving = hangs(doc, piece, bend.into.0).expect("the handle is on the piece");
    assert_eq!(arriving.side, Side::Into);
    assert_ne!(arriving.node, bend.node);
}

#[test]
fn a_node_carries_the_handles_that_hang_from_it() {
    let (draft, piece) = front();
    let doc = draft.doc();
    let bend = bends(&draft, piece)[0];
    assert_eq!(hanging(doc, piece, bend.node), vec![bend.out.0]);
    let waist = doc
        .shows_label(piece, "cintura_cf")
        .expect("the block names its waist");
    assert!(hanging(doc, piece, waist).is_empty());
}

#[test]
fn bending_a_tract_puts_the_handles_on_the_thirds_of_the_chord() {
    let (draft, piece) = front();
    let doc = draft.doc();
    let waist = doc
        .shows_label(piece, "cintura_cf")
        .expect("the block names its waist");
    let commands = bend(doc, piece, waist, ([0.0, 0.0], [30.0, 0.0]));
    assert_eq!(commands.len(), 2);
    // The count first: the document will not hang handles on a tract sampled
    // at one point, because a curve flattened at one sample is its chord.
    assert_eq!(
        commands[0],
        Command::SetSamples {
            piece,
            node: waist,
            to: SAMPLES
        }
    );
    let Command::SetSegment {
        to: SegmentEdit::Cubic(handles),
        ..
    } = &commands[1]
    else {
        panic!("the second command draws the curve: {:?}", commands[1]);
    };
    assert_eq!(handles.out.value, Point::at(10.0, 0.0));
    assert_eq!(handles.into.value, Point::at(20.0, 0.0));
}

#[test]
fn only_a_bent_tract_reports_a_sample_count() {
    let (draft, piece) = front();
    let doc = draft.doc();
    let bend = bends(&draft, piece)[0];
    assert_eq!(samples_of(doc, piece, bend.node), Some(24));
    let waist = doc
        .shows_label(piece, "cintura_cf")
        .expect("the block names its waist");
    assert_eq!(samples_of(doc, piece, waist), None);
}

#[test]
fn handles_are_shown_only_around_what_is_chosen() {
    let (draft, piece) = front();
    let bends = bends(&draft, piece);
    assert!(handles(&bends, &Selection::None).is_empty());
    let bend = bends[0];
    let around = handles(&bends, &Selection::point(bend.node));
    assert_eq!(around.len(), 2);
    let grabbed = handles(&bends, &Selection::point(bend.out.0));
    assert_eq!(grabbed, around, "a handle in hand keeps its own pair lit");
    let chosen = handles(&bends, &Selection::Edge(bend.node));
    assert_eq!(chosen, around);
    assert_eq!(at(&bends, bend.into.0), Some(bend.into.1));
    assert_eq!(at(&bends, bend.node), None, "a node is not a handle");
}
