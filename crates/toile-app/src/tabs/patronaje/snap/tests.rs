#![allow(
    clippy::float_cmp,
    reason = "a position that snapped is the candidate's own, to the bit"
)]

use super::*;

/// A square, in contour order, at a scale where a screen point is a tenth
/// of a centimetre.
const SCALE: f64 = 10.0;

fn square() -> Vec<(PointKey, [f64; 2])> {
    [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]
        .into_iter()
        .enumerate()
        .map(|(i, at)| (PointKey::new(i as u32, 0), at))
        .collect()
}

fn context<'a>(
    nodes: &'a [(PointKey, [f64; 2])],
    tracts: &'a [Tract],
    scale: f64,
) -> SnapContext<'a> {
    SnapContext {
        nodes,
        handles: &[],
        tracts,
        held: &[],
        anchor: [0.0, 0.0],
        scale,
    }
}

#[test]
fn a_node_wins_over_the_grid_within_the_budget() {
    let nodes = square();
    let tracts = tract::straight(&nodes);
    let caught = resolve(
        [10.3, 0.2],
        &context(&nodes, &tracts, SCALE),
        SnapConfig::default(),
    );
    assert_eq!(caught.at, [10.0, 0.0]);
    assert_eq!(caught.kind, Some(SnapKind::Node(nodes[1].0)));
}

#[test]
fn the_grid_catches_what_the_piece_does_not() {
    let nodes = square();
    let tracts = tract::straight(&nodes);
    let caught = resolve(
        [4.05, 5.02],
        &context(&nodes, &tracts, SCALE),
        SnapConfig::default(),
    );
    assert_eq!(caught.at, [4.0, 5.0]);
    assert_eq!(caught.kind, Some(SnapKind::Grid));
    let free = resolve(
        [4.5, 5.5],
        &context(&nodes, &tracts, 30.0),
        SnapConfig::default(),
    );
    assert_eq!(free.at, [4.5, 5.5], "half a centimetre is out of reach");
    assert_eq!(free.kind, None);
}

#[test]
fn a_handle_on_show_is_caught_after_the_node_and_before_the_tract() {
    let nodes = square();
    let tracts = tract::straight(&nodes);
    let handle = [(PointKey::new(9, 0), [5.0, 0.4])];
    let ctx = SnapContext {
        handles: &handle,
        ..context(&nodes, &tracts, SCALE)
    };
    let cfg = SnapConfig {
        grid_cm: 0.0,
        ..SnapConfig::default()
    };
    let caught = resolve([5.2, 0.5], &ctx, cfg);
    assert_eq!(caught.at, [5.0, 0.4]);
    assert_eq!(caught.kind, Some(SnapKind::Handle(handle[0].0)));
    // A node still wins: the ladder is node, then handle, then tract.
    let over = resolve([10.1, 0.1], &ctx, cfg);
    assert_eq!(over.kind, Some(SnapKind::Node(nodes[1].0)));
    // And out of reach of the handle, the tract underneath catches it.
    let along = resolve([2.0, 0.2], &ctx, cfg);
    assert_eq!(
        along.kind,
        Some(SnapKind::Edge {
            from: nodes[0].0,
            t: 0.2
        })
    );
}

#[test]
fn a_tract_catches_between_two_nodes() {
    let nodes = square();
    let tracts = tract::straight(&nodes);
    let cfg = SnapConfig {
        grid_cm: 0.0,
        ..SnapConfig::default()
    };
    let caught = resolve([4.5, 0.2], &context(&nodes, &tracts, SCALE), cfg);
    assert_eq!(caught.at, [4.5, 0.0]);
    assert_eq!(
        caught.kind,
        Some(SnapKind::Edge {
            from: nodes[0].0,
            t: 0.45
        })
    );
}

#[test]
fn the_snap_radius_is_in_screen_points_not_centimetres() {
    let nodes = square();
    let tracts = tract::straight(&nodes);
    let cfg = SnapConfig {
        grid_cm: 0.0,
        ..SnapConfig::default()
    };
    let near = resolve([10.5, 0.0], &context(&nodes, &tracts, SCALE), cfg);
    assert_eq!(near.kind, Some(SnapKind::Node(nodes[1].0)));
    let zoomed = resolve([10.5, 0.0], &context(&nodes, &tracts, 100.0), cfg);
    assert_eq!(
        zoomed.at,
        [10.5, 0.0],
        "ten times in, half a centimetre is far"
    );
    assert_eq!(zoomed.kind, None);
}

#[test]
fn ctrl_suppresses_every_candidate() {
    let nodes = square();
    let tracts = tract::straight(&nodes);
    let cfg = SnapConfig {
        on: false,
        ..SnapConfig::default()
    };
    let free = resolve([10.05, 0.05], &context(&nodes, &tracts, SCALE), cfg);
    assert_eq!(free.at, [10.05, 0.05]);
    assert_eq!(free.kind, None);
}

#[test]
fn shift_constrains_to_the_axis_of_the_anchor() {
    let nodes = square();
    let tracts = tract::straight(&nodes);
    let cfg = SnapConfig {
        axis: true,
        on: false,
        ..SnapConfig::default()
    };
    let ctx = SnapContext {
        anchor: [4.0, 4.0],
        ..context(&nodes, &tracts, SCALE)
    };
    let flat = resolve([9.0, 4.6], &ctx, cfg);
    assert_eq!(flat.at, [9.0, 4.0]);
    assert_eq!(flat.kind, Some(SnapKind::Axis));
    let upright = resolve([4.4, 9.0], &ctx, cfg);
    assert_eq!(upright.at, [4.0, 9.0]);
    let slanted = resolve([9.0, 9.2], &ctx, cfg);
    assert!(
        (slanted.at[0] - slanted.at[1]).abs() < 1.0e-9,
        "{slanted:?}"
    );
}

#[test]
fn an_axis_still_gives_way_to_a_node() {
    let nodes = square();
    let tracts = tract::straight(&nodes);
    let cfg = SnapConfig {
        axis: true,
        ..SnapConfig::default()
    };
    let ctx = SnapContext {
        anchor: [0.0, 0.0],
        ..context(&nodes, &tracts, SCALE)
    };
    let caught = resolve([10.2, 0.0], &ctx, cfg);
    assert_eq!(caught.kind, Some(SnapKind::Node(nodes[1].0)));
}
