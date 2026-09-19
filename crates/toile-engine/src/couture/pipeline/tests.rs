use super::*;

/// A rectangle, corners only: cheap to mesh and easy to edit.
fn rectangle() -> Vec<[f64; 2]> {
    vec![[0.0, 0.0], [0.30, 0.0], [0.30, 0.20], [0.0, 0.20]]
}

fn pipeline() -> ShapePipeline {
    ShapePipeline::build(&rectangle(), 16, 0.01).expect("the rectangle is finite")
}

#[test]
fn derive_with_a_different_point_count_is_an_error() {
    let mut pipe = pipeline();
    let mut grown = rectangle();
    grown.push([0.15, 0.30]);
    assert_eq!(
        pipe.derive(&grown),
        Err(RestStateError::PointCount {
            expected: 4,
            got: 5
        })
    );
}

#[test]
fn a_moved_node_keeps_the_mesh_and_changes_the_rest_lengths() {
    let mut pipe = pipeline();
    let before = pipe.rests.clone();
    let mut edited = rectangle();
    edited[1][0] += 0.05;
    let after = pipe.derive(&edited).expect("the node count did not move");
    assert_eq!(after.len(), before.len());
    assert_ne!(after, before.as_slice());
    assert_eq!(pipe.contour_len(), 4);
}

/// A stretch of contour comes back as the boundary vertices under it, in the
/// order the contour walks them, and every consecutive pair is an edge of the
/// mesh. That last clause is the whole of what an elastic needs: it holds the
/// edges along a stretch, and a walk that came back out of order would name
/// chords across the piece instead.
#[test]
fn a_stretch_of_contour_walks_the_boundary_edges_under_it() {
    let pipe = pipeline();
    let along = pipe.boundary_run((0.0, 0.3));
    assert!(along.len() >= 4, "{} vertices along the hem", along.len());
    let mut last = f64::MIN;
    for &v in &along {
        let p = pipe.pos2d[v as usize];
        assert!(p[1].abs() < 1.0e-9, "vertex {v} is off the hem at {p:?}");
        assert!(p[0] > last, "and they come back in order");
        last = p[0];
    }
    for pair in along.windows(2) {
        assert!(pipe.edge_index(pair[0], pair[1]).is_some(), "{pair:?}");
    }
    assert_eq!(pipe.edge_index(0, u32::MAX), None, "and no other pair is");
}

/// A stretch that passes the closure is one walk and not two halves in the
/// wrong order.
#[test]
fn a_stretch_over_the_closure_comes_back_in_one_piece() {
    let pipe = pipeline();
    let over = pipe.boundary_run((0.9, 0.2));
    assert!(over.len() >= 3, "{} vertices round the corner", over.len());
    let (first, last) = (
        pipe.pos2d[over[0] as usize],
        pipe.pos2d[over[over.len() - 1] as usize],
    );
    assert!(
        first[0].abs() < 1.0e-9 && first[1] > 0.0,
        "it starts up the side"
    );
    assert!(
        last[1].abs() < 1.0e-9 && last[0] > 0.0,
        "and ends along the hem"
    );
    for pair in over.windows(2) {
        assert!(pipe.edge_index(pair[0], pair[1]).is_some(), "{pair:?}");
    }
}

#[test]
fn a_contour_the_mesher_refuses_is_an_error_not_a_panic() {
    let mut broken = rectangle();
    broken[2][0] = f64::NAN;
    assert_eq!(
        ShapePipeline::build(&broken, 16, 0.01).err(),
        Some(MeshError::NonFiniteVertex { index: 0 })
    );
}
