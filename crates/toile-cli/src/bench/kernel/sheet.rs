use toile_sim::xpbd::{DistanceConstraints, SdfGrid, State};

use crate::bench::scene::{Lcg, shuffle};

/// A flat sheet of roughly `target` vertices at 5 mm particle distance,
/// falling onto a 15 cm sphere.
pub(super) struct Scene {
    pub(super) state: State,
    pub(super) cons: DistanceConstraints,
    pub(super) tris: Vec<u32>,
    pub(super) sdf: SdfGrid,
}

/// Builds the sheet with its vertex numbering and constraint order shuffled.
///
/// The shuffle is the point: a real CDT mesh leaves scattered gather/scatter,
/// and a benchmark over a densely ordered grid would report a speed the
/// product never sees.
pub(super) fn build(target: usize) -> Scene {
    const SPACING: f32 = 0.005;
    const C_STRUCT: f32 = 1.0e-8;
    const C_SHEAR: f32 = 5.0e-7;
    const C_BEND: f32 = 1.0e-5;

    let w = (target as f64).sqrt() as usize;
    let h = target / w;
    let n = w * h;
    let mut rng = Lcg(0x0005_EED7_011E);

    let mut perm: Vec<u32> = (0..n as u32).collect();
    shuffle(&mut perm, &mut rng);

    let mut state = State::new(n);
    let (ox, oz) = (w as f32 * SPACING * 0.5, h as f32 * SPACING * 0.5);
    for j in 0..h {
        for i in 0..w {
            let v = perm[j * w + i] as usize;
            state.px[v] = i as f32 * SPACING - ox;
            state.py[v] = 0.3;
            state.pz[v] = j as f32 * SPACING - oz;
        }
    }

    let mut edges: Vec<(u32, u32, f32)> = Vec::new();
    let mut link = |i2: usize, j2: usize, i: usize, j: usize, c: f32| {
        edges.push((perm[j * w + i], perm[j2 * w + i2], c));
    };
    for j in 0..h {
        for i in 0..w {
            if i + 1 < w {
                link(i + 1, j, i, j, C_STRUCT);
            }
            if j + 1 < h {
                link(i, j + 1, i, j, C_STRUCT);
            }
            if i + 1 < w && j + 1 < h {
                link(i + 1, j + 1, i, j, C_SHEAR);
                link(i, j + 1, i + 1, j, C_SHEAR);
            }
            if i + 2 < w {
                link(i + 2, j, i, j, C_BEND);
            }
            if j + 2 < h {
                link(i, j + 2, i, j, C_BEND);
            }
        }
    }
    shuffle(&mut edges, &mut rng);

    let mut cons = DistanceConstraints {
        a: Vec::with_capacity(edges.len()),
        b: Vec::with_capacity(edges.len()),
        rest: Vec::with_capacity(edges.len()),
        compliance: Vec::with_capacity(edges.len()),
        ..DistanceConstraints::default()
    };
    for (a, b, c) in edges {
        let (ia, ib) = (a as usize, b as usize);
        let (dx, dy, dz) = (
            state.px[ib] - state.px[ia],
            state.py[ib] - state.py[ia],
            state.pz[ib] - state.pz[ia],
        );
        cons.a.push(a);
        cons.b.push(b);
        cons.rest.push((dx * dx + dy * dy + dz * dz).sqrt());
        cons.compliance.push(c);
    }

    let mut tris = Vec::with_capacity((w - 1) * (h - 1) * 6);
    for j in 0..h - 1 {
        for i in 0..w - 1 {
            let (a, b, c, d) = (
                perm[j * w + i],
                perm[j * w + i + 1],
                perm[(j + 1) * w + i],
                perm[(j + 1) * w + i + 1],
            );
            tris.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }

    Scene {
        state,
        cons,
        tris,
        sdf: toile_engine::demo::avatar_sdf(),
    }
}
