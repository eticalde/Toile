use super::*;
use crate::xpbd::{
    DistanceConstraints, Floor, Layers, SdfGrid, State, color_constraints, position_hash,
    substep_colored, substep_colored_simd,
};

const N: usize = 12;
const SPACING: f32 = 0.01;
const DT: f32 = 1.0 / 600.0;

/// A flat sheet of `N`×`N` particles with its structural and shear edges.
fn sheet() -> (State, DistanceConstraints, Vec<u32>) {
    let mut state = State::new(N * N);
    for j in 0..N {
        for i in 0..N {
            let v = j * N + i;
            state.px[v] = i as f32 * SPACING;
            state.py[v] = 0.1;
            state.pz[v] = j as f32 * SPACING;
        }
    }
    let mut cons = DistanceConstraints::default();
    let link = |cons: &mut DistanceConstraints, a: usize, b: usize| {
        let (dx, dz) = (state.px[b] - state.px[a], state.pz[b] - state.pz[a]);
        let d = (dx * dx + dz * dz).sqrt();
        cons.a.push(a as u32);
        cons.b.push(b as u32);
        cons.rest.push(d);
        cons.compliance.push(1.0e-8);
    };
    let mut tris = Vec::new();
    for j in 0..N {
        for i in 0..N {
            let v = j * N + i;
            if i + 1 < N {
                link(&mut cons, v, v + 1);
            }
            if j + 1 < N {
                link(&mut cons, v, v + N);
            }
            if i + 1 < N && j + 1 < N {
                link(&mut cons, v, v + N + 1);
                tris.extend_from_slice(&[v as u32, (v + N) as u32, (v + 1) as u32]);
            }
        }
    }
    (state, cons, tris)
}

/// A field the sheet never reaches, so what these tests compare is the solve
/// and not the contact the two paths already share.
fn field() -> SdfGrid {
    SdfGrid::sphere(16, 0.02, [-0.16, -0.16, -0.16], [0.0, 0.0, 0.0], 0.05)
}

#[test]
fn a_scene_of_nothing_but_cloth_and_a_body_is_taken() {
    let (mut state, cons, _) = sheet();
    let cc = color_constraints(&cons, N * N).expect("a plain stretch set colours");
    let sdf = field();
    let void = Stage::around(&sdf);
    let none = Seams::default();
    assert_eq!(
        substep_colored(&mut state, &cc, &none, &void, None, DT),
        Ok(())
    );
    assert_eq!(
        substep_colored_simd(&mut state, &cc, &none, &void, None, DT),
        Ok(())
    );
}

/// The trap two reviews named: a caller holding any of these could hand it to
/// a path that integrates straight past it.
///
/// Both paths are asked, because both take the scene, and the positions are
/// read either side of the refusal: a pass that is dropped and a pass that is
/// half run are the same defect to whoever is wearing the garment.
#[test]
fn every_pass_the_coloured_paths_lack_refuses_the_scene() {
    let (mut state, cons, tris) = sheet();
    let cc = color_constraints(&cons, N * N).expect("a plain stretch set colours");
    let sdf = field();
    let plain = Stage::around(&sdf);
    let sewn = Seams {
        a: vec![0],
        b: vec![1],
        ..Seams::default()
    };
    let layers = Layers::of(&tris, &cons, &Seams::default(), N * N);
    let none = Seams::default();
    for (seams, stage, over, want) in [
        (&sewn, plain, None, Dropped::Seams),
        (&none, plain, Some(&layers), Dropped::Layers),
        (&none, plain.on(Floor::at(0.0)), None, Dropped::Floor),
        (
            &none,
            plain.holding(Grip::coulomb(0.6, 0.4)),
            None,
            Dropped::Grip,
        ),
        (&none, plain.weightless(), None, Dropped::Gravity),
    ] {
        let before = position_hash(&state);
        assert_eq!(
            substep_colored(&mut state, &cc, seams, &stage, over, DT),
            Err(want)
        );
        assert_eq!(
            substep_colored_simd(&mut state, &cc, seams, &stage, over, DT),
            Err(want)
        );
        assert_eq!(position_hash(&state), before, "a refusal moves nothing");
    }
}

/// Refused at the colouring and not at the substep, because the permutation is
/// what a held index cannot survive.
#[test]
fn an_elastic_and_a_strain_limit_cannot_be_coloured_at_all() {
    let (_, plain, _) = sheet();
    let limited = DistanceConstraints {
        strain_limit: 1.03,
        strain_sweeps: 4,
        ..plain.clone()
    };
    let elastic = DistanceConstraints {
        held: vec![0, 1],
        held_passes: 2,
        ..plain
    };
    assert_eq!(
        color_constraints(&limited, N * N).err(),
        Some(Dropped::StrainLimit)
    );
    assert_eq!(
        color_constraints(&elastic, N * N).err(),
        Some(Dropped::Held)
    );
}

/// The claim the SIMD path's doc makes, as a gate rather than as prose.
///
/// A colour of this sheet holds well over eight constraints, so the batched
/// arithmetic is what is being compared and not the scalar tail that follows
/// it.
#[test]
fn the_simd_path_matches_the_coloured_path_bit_for_bit() {
    let (mut wide, cons, _) = sheet();
    let cc = color_constraints(&cons, N * N).expect("a plain stretch set colours");
    let sdf = field();
    let void = Stage::around(&sdf);
    let none = Seams::default();
    let widest = cc.ranges.iter().map(ExactSizeIterator::len).max();
    assert!(widest.is_some_and(|w| w >= 8), "the batches must run");

    let mut lanes = wide.clone();
    for _ in 0..30 {
        substep_colored(&mut wide, &cc, &none, &void, None, DT).expect("the sheet is bare");
        substep_colored_simd(&mut lanes, &cc, &none, &void, None, DT).expect("the sheet is bare");
    }
    assert_eq!(position_hash(&wide), position_hash(&lanes));
}
