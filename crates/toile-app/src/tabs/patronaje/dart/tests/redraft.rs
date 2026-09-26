use toile_engine::draft::{Binding, Command, Dart, Draft, PointKey};

use super::*;

/// The hem of the block's front: the tract leaving the node at the outer
/// bottom. Both of that node's coordinates are formulas and the tract is
/// straight, which is what a wedge needs and what the waistline cannot give
/// — the centre front is bound to plain numbers.
const HEM: usize = 4;

/// How deep into the cloth the apex of the measured wedge goes, in
/// centimetres.
const DEEP: f64 = 5.0;

/// How far the hem of the block moves between the two bodies it ships with, in
/// centimetres: `largo_lateral` is 104 on one and 106 on the other.
const HEM_MOVES: f64 = 2.0;

/// Whatever the cut is written to, read back to the last hundredth.
const CLOSE: f64 = 0.005;

/// The whole gesture on one straight tract: two legs along it, the apex that
/// far into the cloth from the middle of it.
fn cut_on(table: &Table, tract: usize, legs: (f64, f64)) -> Vec<Command> {
    let ctx = table.wielding(Tool::Dart);
    let (gesture, _, _) = begin(along(table, tract, legs.0), Mods::default(), &ctx);
    let second = down(along(table, tract, legs.1));
    let (gesture, _, _) = update(darting(&gesture), &second, &ctx);
    let (_, commands, _) = update(darting(&gesture), &down(apex_of(table, tract)), &ctx);
    commands
}

/// A place `DEEP` centimetres inside the cloth from the middle of one tract.
///
/// The block's hem is the bottom of the leg, so inside it is up the page.
fn apex_of(table: &Table, tract: usize) -> Pos2 {
    let from = table.nodes[tract].1;
    let to = table.nodes[(tract + 1) % table.nodes.len()].1;
    View::default().to_screen([
        f64::midpoint(from[0], to[0]),
        f64::midpoint(from[1], to[1]) - DEEP,
    ])
}

/// Where one point of the draft lies.
fn at(draft: &Draft, point: PointKey) -> [f64; 2] {
    draft.resolved(point).expect("the key is live")
}

/// The wedge's three nodes and the node it was cut after, resolved.
fn wedge_at(draft: &Draft, dart: &Dart, after: PointKey) -> [[f64; 2]; 4] {
    [
        at(draft, dart.legs.0),
        at(draft, dart.apex),
        at(draft, dart.legs.1),
        at(draft, after),
    ]
}

/// A wedge the tool cuts goes on meaning something when the pattern is
/// re-drafted on another body.
///
/// What it takes out of the contour, where along the tract it takes it and how
/// deep it goes are all the same on the second body, and the legs are still on
/// the hem. The plain numbers the same three presses used to write would have
/// stayed where the first body left them, which the hem no longer runs through.
#[test]
fn a_cut_wedge_travels_with_the_cloth_onto_another_body() {
    let table = table();
    let commands = cut_on(&table, HEM, (0.3, 0.7));
    let (dart, wedge) = asked(&commands);
    for node in &wedge.nodes {
        let stated = |binding: &Binding| matches!(*binding, Binding::Formula(_));
        assert!(
            stated(&node.value.x) && stated(&node.value.y),
            "the wedge is stated from the node it follows: {:?}",
            node.value
        );
    }
    let after = wedge
        .after
        .expect("the wedge follows the node of its tract");
    let mut draft = table.draft.clone();
    for command in commands {
        draft.edit(command).expect("the contour takes the wedge");
    }
    let [leg0, apex, leg1, hem] = wedge_at(&draft, &dart, after);
    let other = draft
        .doc()
        .mannequin_named("Talla 42")
        .expect("the block brings a second body");
    draft
        .edit(Command::ResolveWith { mannequin: other })
        .expect("the block resolves on either body");
    let [moved0, moved_apex, moved1, moved_hem] = wedge_at(&draft, &dart, after);

    let span = |a: [f64; 2], b: [f64; 2]| pick::away(a, b);
    assert!(
        (span(leg0, leg1) - span(moved0, moved1)).abs() < CLOSE,
        "the mouth takes out what it took: {leg0:?} {leg1:?} vs {moved0:?} {moved1:?}"
    );
    assert!(
        (span(leg0, apex) - span(moved0, moved_apex)).abs() < CLOSE,
        "and the apex is as deep as it was"
    );
    assert!(
        (span(hem, leg0) - span(moved_hem, moved0)).abs() < CLOSE,
        "and it sits where along the hem it sat"
    );
    // The hem runs level on either body, so following its node keeps the legs
    // on it exactly rather than nearly.
    assert!((moved0[1] - moved_hem[1]).abs() < CLOSE);
    assert!((moved1[1] - moved_hem[1]).abs() < CLOSE);
    // And the reading the plain numbers of before would have given: the hem is
    // no longer within two centimetres of where they were written.
    assert!(
        (moved_hem[1] - hem[1]).abs() > HEM_MOVES - CLOSE,
        "the cloth moved out from under a plain number: {hem:?} {moved_hem:?}"
    );
}
