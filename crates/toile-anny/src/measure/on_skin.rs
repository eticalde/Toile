use std::collections::BTreeMap;

use super::*;
use crate::body_mesh;
use crate::phenotype::Phenotype;

/// Every triangle each undirected mesh edge belongs to, keyed by the edge's
/// two vertices in ascending order.
fn triangles_by_edge() -> BTreeMap<(u16, u16), Vec<usize>> {
    let mut edges: BTreeMap<(u16, u16), Vec<usize>> = BTreeMap::new();
    for (tri, v) in decoded().indices.as_chunks::<3>().0.iter().enumerate() {
        for (a, b) in [(v[0], v[1]), (v[1], v[2]), (v[2], v[0])] {
            let (a, b) = (a as u16, b as u16);
            edges.entry((a.min(b), a.max(b))).or_default().push(tri);
        }
    }
    edges
}

fn edge_of(p: RingPoint) -> (u16, u16) {
    (p.vertex_a.min(p.vertex_b), p.vertex_a.max(p.vertex_b))
}

/// Every path point sits on a real edge of the mesh, inside it, and every
/// chord between two consecutive points has both ends on the edges of one
/// triangle.
///
/// That is the whole proof the drawn and measured line is on the skin, and on
/// every body: a triangle is flat, so the chord between two points on its
/// border lies in it whatever the phenotype did to its three corners, and the
/// triangles are the surface the body is rendered and draped as. A chord that
/// jumped across the body, or to the other leg, would join two edges that
/// share no triangle.
#[test]
fn every_path_chord_lies_in_one_triangle_of_the_mesh() {
    let edges = triangles_by_edge();
    for id in PathId::ALL {
        let points = path(id);
        assert!(points.len() >= 2, "{id:?} has no chord to measure");
        for &p in points {
            assert!(
                (0.0..=1.0).contains(&p.t),
                "{id:?}: t = {} is off its edge",
                p.t
            );
            assert!(
                edges.contains_key(&edge_of(p)),
                "{id:?}: {p:?} is not on an edge of the mesh"
            );
        }
        for (i, pair) in points.windows(2).enumerate() {
            let shared = edges[&edge_of(pair[0])]
                .iter()
                .any(|tri| edges[&edge_of(pair[1])].contains(tri));
            assert!(shared, "{id:?}: chord {i} crosses no single triangle");
        }
    }
}

/// The bodies the side checks run on: the reference adult, and the corners of
/// the phenotype box a morph can stretch the paths furthest toward.
fn bodies() -> Vec<Vec<f32>> {
    let corner = |gender, weight, muscle| Phenotype {
        gender,
        age: 0.8,
        muscle,
        weight,
        height: 0.5,
        proportions: 0.5,
    };
    [
        corner(0.0, 0.5, 0.5),
        corner(1.0, 1.0, 0.0),
        corner(0.0, 0.0, 1.0),
    ]
    .iter()
    .map(|p| body_mesh(p, &[0.0; 20]).positions)
    .collect()
}

/// How far above its first point the inseam may still reach: the fork is
/// rounded, and a morph that rolls it a little sideways lifts the next point
/// or two by a millimetre. A walk that had gone the wrong way round the
/// section, up the side of the trunk, rises tens of centimetres.
const FORK_ROUNDING_M: f32 = 0.01;

/// The leg and arm paths never leave the body's right side, the inseam never
/// climbs back up past the fork it starts at, and the back stays on the
/// mirror plane.
#[test]
fn no_path_crosses_to_the_other_side_of_the_body() {
    for positions in bodies() {
        let points = |id| {
            path(id)
                .iter()
                .map(|&p| at(&positions, p))
                .collect::<Vec<_>>()
        };
        for id in [
            PathId::Inseam,
            PathId::Outseam,
            PathId::Rise,
            PathId::HipDrop,
            PathId::Arm,
        ] {
            for q in points(id) {
                assert!(q[0] < 0.0, "{id:?} reaches {q:?}, left of the midline");
            }
        }
        let inseam = points(PathId::Inseam);
        for q in &inseam {
            assert!(
                q[1] <= inseam[0][1] + FORK_ROUNDING_M,
                "the inseam climbs to {q:?}"
            );
        }
        for q in points(PathId::Back) {
            assert!(q[0].abs() < 1.0e-3, "the back path strays to {q:?}");
        }
        let across = points(PathId::Shoulders);
        let sides = across
            .windows(2)
            .filter(|w| (w[0][0] < 0.0) != (w[1][0] < 0.0));
        assert_eq!(
            sides.count(),
            1,
            "the shoulders path crosses the spine once"
        );
    }
}

/// `tiro` and `altura_cadera` are the first stretch of `largo_lateral`'s own
/// line, so the three can never disagree about where the side of the body is.
#[test]
fn the_rise_and_the_hip_drop_are_the_start_of_the_outseam() {
    let outseam = path(PathId::Outseam);
    let (rise, hip) = (path(PathId::Rise), path(PathId::HipDrop));
    assert!(hip.len() < rise.len() && rise.len() < outseam.len());
    assert_eq!(rise, &outseam[..rise.len()]);
    assert_eq!(hip, &outseam[..hip.len()]);
}
