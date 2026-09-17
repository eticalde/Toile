use toile_sim::xpbd::{DistanceConstraints, State};

use super::pipeline::ShapePipeline;
use super::place::Layout;
use super::seed;

/// Where each piece's vertices begin in the combined solver state.
///
/// A product is one state holding every piece, concatenated in the order they
/// are given, so a piece is addressed past every piece before it. The first
/// begins at zero. A seam joins two vertices of that one state, which is why
/// each side of one has to be offset by its own piece's base.
pub fn offsets(pipelines: &[&ShapePipeline]) -> Vec<u32> {
    let mut at = 0;
    let mut bases = Vec::with_capacity(pipelines.len());
    for pipe in pipelines {
        bases.push(at);
        at += pipe.pos2d.len() as u32;
    }
    bases
}

/// Seeds the solver with every piece of a product, concatenated.
///
/// A piece the seams place is rolled onto the ring `around` names, at the
/// height that ring sits on the body; `height` is no part of it. A piece they
/// do not — a lone panel, which has no partner to be placed against — is let go
/// exactly as one piece is let go on its own: centred on its own vertices and
/// released flat at `height`. That second branch is the arithmetic the drape
/// goldens hash, reached with nothing added to anything.
pub fn drop_all(pipelines: &[&ShapePipeline], height: f32, around: Option<&Layout>) -> State {
    let mut state = State::new(pipelines.iter().map(|pipe| pipe.pos2d.len()).sum());
    let mut base = 0;
    for (at, pipe) in pipelines.iter().enumerate() {
        let placed = around.is_some_and(|layout| layout.wrap_into(&mut state, base, pipe, at));
        if !placed {
            seed::release_into(&mut state, base, pipe, height);
        }
        base += pipe.pos2d.len();
    }
    state
}

/// Concatenates every piece's stretch constraints into the one set the solver
/// holds, each piece's endpoints rebased past the pieces before it.
pub fn combine_constraints(pipelines: &[&ShapePipeline], compliance: f32) -> DistanceConstraints {
    let mut all = DistanceConstraints::default();
    let mut base = 0u32;
    for pipe in pipelines {
        let one = pipe.constraints(compliance);
        all.a.extend(one.a.iter().map(|&v| v + base));
        all.b.extend(one.b.iter().map(|&v| v + base));
        all.rest.extend_from_slice(&one.rest);
        all.compliance.extend_from_slice(&one.compliance);
        base += pipe.pos2d.len() as u32;
    }
    all
}

/// Concatenates every piece's triangles into the one list that indexes the
/// combined state, each rebased past the pieces before it.
///
/// The same arithmetic the viewer already does to put the body's triangles
/// behind the cloth's, which is why it is worth doing it the one way.
pub fn combine_triangles(pipelines: &[&ShapePipeline]) -> Vec<u32> {
    let mut tris = Vec::new();
    let mut base = 0u32;
    for pipe in pipelines {
        tris.extend(pipe.tris.iter().map(|&v| v + base));
        base += pipe.pos2d.len() as u32;
    }
    tris
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "a concatenation hands back the very bits it was given"
    )]

    use toile_sim::xpbd::position_hash;

    use super::super::pipeline::COMPLIANCE;
    use super::super::seed::{DROP_HEIGHT, drop_state};
    use super::*;

    fn pipeline(w: f64, h: f64) -> ShapePipeline {
        let rectangle = [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]];
        ShapePipeline::build(&rectangle, 16, 0.01).expect("the rectangle is finite")
    }

    /// The reduction the whole many-piece path stands on.
    ///
    /// A product of one piece is that piece's own arithmetic with an offset of
    /// zero added to nothing, not something that merely rounds to it. The
    /// drape goldens are one piece, and this is what says they cannot move.
    #[test]
    fn a_product_of_one_piece_is_that_piece_exactly() {
        let pipe = pipeline(0.30, 0.20);
        let one = [&pipe];
        assert_eq!(offsets(&one), [0]);
        assert_eq!(combine_triangles(&one), pipe.tris);

        let alone = pipe.constraints(COMPLIANCE);
        let combined = combine_constraints(&one, COMPLIANCE);
        assert_eq!(combined.a, alone.a);
        assert_eq!(combined.b, alone.b);
        assert_eq!(combined.rest, alone.rest);
        assert_eq!(combined.compliance, alone.compliance);
        assert_eq!(combined.strain_limit, alone.strain_limit);
        assert_eq!(combined.strain_sweeps, alone.strain_sweeps);

        assert_eq!(
            position_hash(&drop_all(&one, DROP_HEIGHT, None)),
            position_hash(&drop_state(&pipe, DROP_HEIGHT)),
            "the seeding the drape golden hashes has to come back bit for bit"
        );
    }

    /// A piece the layout does not place falls back to the flat release, bit
    /// for bit — which is what lets a product hold a panel nothing is sewn to
    /// without the placed pieces disturbing it.
    #[test]
    fn a_layout_that_places_nothing_seeds_the_flat_release() {
        let pipe = pipeline(0.30, 0.20);
        let one = [&pipe];
        let empty = Layout {
            axis: [0.4, -0.3],
            radius: 0.5,
            stand: 0.9,
            crest: 0.2,
            wraps: vec![None],
        };
        assert_eq!(
            position_hash(&drop_all(&one, DROP_HEIGHT, Some(&empty))),
            position_hash(&drop_state(&pipe, DROP_HEIGHT))
        );
    }

    /// And one that does place it puts every vertex on the cylinder instead.
    #[test]
    fn a_placed_piece_is_rolled_onto_the_stand() {
        let pipe = pipeline(0.30, 0.20);
        let around = Layout {
            axis: [0.0, 0.0],
            radius: 0.5,
            stand: DROP_HEIGHT,
            crest: 0.20,
            wraps: vec![Some(super::super::place::Wrap {
                turn: 0.0,
                sense: 1.0,
            })],
        };
        let state = drop_all(&[&pipe], DROP_HEIGHT, Some(&around));
        for i in 0..state.len() {
            let r = f64::from(state.px[i].hypot(state.pz[i]));
            assert!((r - around.radius).abs() < 1.0e-6, "vertex {i} at {r}");
        }
    }

    /// A second piece is the first's arithmetic at its own base: the same
    /// positions, the same rest lengths, every index moved by the vertices
    /// standing before it and by nothing else.
    #[test]
    fn a_second_piece_is_the_same_arithmetic_at_its_own_offset() {
        let (front, back) = (pipeline(0.30, 0.20), pipeline(0.22, 0.26));
        let both = [&front, &back];
        let na = front.pos2d.len() as u32;
        assert_eq!(offsets(&both), [0, na]);

        let tris = combine_triangles(&both);
        let rebased: Vec<u32> = back.tris.iter().map(|&v| v + na).collect();
        assert_eq!(tris[..front.tris.len()], front.tris[..]);
        assert_eq!(tris[front.tris.len()..], rebased[..]);

        let edges = front.edges.len();
        let cons = combine_constraints(&both, COMPLIANCE);
        let alone = back.constraints(COMPLIANCE);
        assert_eq!(cons.len(), edges + alone.len());
        assert!(cons.a[..edges].iter().all(|&v| v < na));
        assert!(cons.b[edges..].iter().all(|&v| v >= na));
        assert_eq!(cons.rest[edges..], alone.rest[..]);

        let state = drop_all(&both, DROP_HEIGHT, None);
        let dropped = drop_state(&back, DROP_HEIGHT);
        assert_eq!(state.len(), front.pos2d.len() + back.pos2d.len());
        assert_eq!(state.px[na as usize..], dropped.px[..]);
        assert_eq!(state.pz[na as usize..], dropped.pz[..]);
    }
}
