#![allow(
    clippy::float_cmp,
    reason = "a pass that touched nothing hands back the very bits it was given"
)]

use super::*;
use crate::xpbd::contact::Floor;
use crate::xpbd::crossings::self_crossings;
use crate::xpbd::metrics::position_hash;
use crate::xpbd::sdf::SdfGrid;
use crate::xpbd::solver::substep;

/// Simulated seconds per substep, as the engine runs it.
const DT: f32 = 1.0 / 600.0;

/// Substeps every scene here is read at: two and a half simulated seconds.
const STEPS: usize = 1500;

/// Substeps between two readings of the crossing count.
const WATCH: usize = 5;

/// Particle spacing of every sheet here, in metres.
const SPACING: f32 = 0.01;

/// How far apart two layers are seeded, in metres. Well inside the thickness,
/// so the pass has to part them from the first substep.
const GAP: f32 = 0.002;

/// Cosine and sine of the angle the fold below starts open at.
///
/// A three-four-five triangle, so the leaf opens by a real angle without an
/// angle function and every rest length across the fold is still satisfied.
/// It has to start open: a leaf lowered flat onto the leaf under it sinks
/// through it tangentially, and a surface that crosses itself tangentially
/// has no edge piercing any triangle to be counted.
const OPEN: [f32; 2] = [0.8, 0.6];

/// A flat sheet in the xz plane, and its triangles.
fn sheet(w: usize, h: usize, at: [f32; 3]) -> (State, Vec<u32>) {
    let mut state = State::new(w * h);
    for j in 0..h {
        for i in 0..w {
            let v = j * w + i;
            state.px[v] = at[0] + i as f32 * SPACING;
            state.py[v] = at[1];
            state.pz[v] = at[2] + j as f32 * SPACING;
        }
    }
    let mut tris = Vec::with_capacity((w - 1) * (h - 1) * 6);
    for j in 0..h - 1 {
        for i in 0..w - 1 {
            let (a, b) = ((j * w + i) as u32, (j * w + i + 1) as u32);
            let (c, d) = (((j + 1) * w + i) as u32, ((j + 1) * w + i + 1) as u32);
            tris.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    (state, tris)
}

/// Every edge the triangles draw, once, at the length it has right now.
///
/// Rest lengths are read off the flat layout, which is what makes the folded
/// scene below a fold in cloth rather than cloth whose rest shape is a fold.
fn stretch(state: &State, tris: &[u32]) -> DistanceConstraints {
    let mut pairs: Vec<(u32, u32)> = Vec::with_capacity(tris.len());
    for t in tris.as_chunks::<3>().0 {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            pairs.push((a.min(b), a.max(b)));
        }
    }
    pairs.sort_unstable();
    pairs.dedup();
    let mut cons = DistanceConstraints::default();
    for (a, b) in pairs {
        let (ia, ib) = (a as usize, b as usize);
        let (dx, dy, dz) = (
            state.px[ib] - state.px[ia],
            state.py[ib] - state.py[ia],
            state.pz[ib] - state.pz[ia],
        );
        cons.a.push(a);
        cons.b.push(b);
        cons.rest.push((dx * dx + dy * dy + dz * dz).sqrt());
        cons.compliance.push(1.0e-8);
    }
    cons
}

/// A field far enough away that nothing in these scenes touches it.
fn nowhere() -> SdfGrid {
    SdfGrid::sphere(8, 0.5, [-10.0, -10.0, -10.0], [-8.0, -8.0, -8.0], 0.1)
}

/// Mean height of a run of vertices, in metres.
fn height(state: &State, run: std::ops::Range<usize>) -> f32 {
    let n = run.len() as f32;
    run.map(|v| state.py[v]).sum::<f32>() / n
}

/// One layer of cloth never collides with itself.
///
/// The thickness has to clear every pair the mesh holds close on purpose, and
/// the tightest of those is the far corner of a split quad. If it does not, a
/// panel hanging under its own weight pushes itself into ripples and the hash
/// says so. This is also the goldens' promise read from the other side: a
/// pass that finds nothing has to leave the arithmetic exactly as it was.
#[test]
fn one_layer_of_cloth_never_collides_with_itself() {
    let (mut alone, tris) = sheet(16, 16, [-0.08, 0.30, -0.08]);
    let cons = stretch(&alone, &tris);
    for v in 0..16 {
        alone.inv_mass[v] = 0.0;
    }
    let mut watched = alone.clone();
    let mut layers = Layers::of(&tris, &cons, &Seams::default(), watched.len());
    let (sdf, seams) = (nowhere(), Seams::default());
    for _ in 0..STEPS {
        substep(&mut alone, &cons, &seams, &sdf, Floor::none(), None, DT);
        let on = Some(&mut layers);
        substep(&mut watched, &cons, &seams, &sdf, Floor::none(), on, DT);
    }
    assert_eq!(
        layers.contacts(),
        0,
        "a banner hanging flat parted itself {} times: the thickness is \
         reaching pairs the mesh already holds",
        layers.contacts()
    );
    assert_eq!(
        position_hash(&alone),
        position_hash(&watched),
        "a pass that parted nothing is not a pass that rounds: it is no pass"
    );
}

/// A fold in cloth is the failure this pass exists for.
///
/// A sheet doubled back on itself: the long half pinned flat, the short half
/// standing open above it and hinged along the crease, closing onto it like
/// the cover of a book. Left alone the upper leaf goes clean through the
/// lower one and ends up hanging underneath it.
///
/// Read through the whole drape and not only at the end, because a leaf that
/// has finished passing through is on the far side and crosses nothing there.
/// Both numbers are reported: a crossing count that was already zero before
/// the pass ran proves nothing about the pass.
#[test]
fn a_fold_stops_falling_through_itself() {
    let (w, h, crease) = (10usize, 20usize, 14usize);
    let (flat, tris) = sheet(w, h, [-0.045, 0.10, -0.095]);
    let cons = stretch(&flat, &tris);
    let mut open = folded(&flat, w, h, crease);
    let mut held = open.clone();
    let mut layers = Layers::of(&tris, &cons, &Seams::default(), held.len());
    let (sdf, seams) = (nowhere(), Seams::default());
    let (mut loose, mut kept) = (0usize, 0usize);
    for step in 0..STEPS {
        substep(&mut open, &cons, &seams, &sdf, Floor::none(), None, DT);
        let on = Some(&mut layers);
        substep(&mut held, &cons, &seams, &sdf, Floor::none(), on, DT);
        if step % WATCH == 0 {
            loose = loose.max(self_crossings(&open, &tris));
            kept = kept.max(self_crossings(&held, &tris));
        }
    }

    let leaf = (crease + 1) * w..w * h;
    let under = height(&open, leaf.clone());
    let over = height(&held, leaf);
    println!(
        "fold: worst {loose} crossings loose, {kept} held · upper leaf ends at \
         {under:.4} loose and {over:.4} held · {} contacts parted",
        layers.contacts()
    );
    assert!(
        loose > 0,
        "the scene has to fold through itself, or it is no test"
    );
    assert!(
        kept * 4 < loose,
        "the pass has to stop it: {kept} of {loose}"
    );
    assert!(
        over > under + GAP,
        "and the upper leaf has to end on top: {over} against {under}"
    );
}

/// The same sheet doubled back on itself, its lower leaf pinned and its upper
/// leaf standing open at [`OPEN`].
///
/// Every row of the upper leaf is laid at its own distance from the crease
/// along the open direction, so the fold is a rotation and not a stretch:
/// what is under test is two layers meeting, not a mesh under strain.
fn folded(flat: &State, w: usize, h: usize, crease: usize) -> State {
    let mut state = flat.clone();
    let (hinge_z, hinge_y) = (flat.pz[crease * w], flat.py[crease * w]);
    for v in 0..(crease + 1) * w {
        state.inv_mass[v] = 0.0;
    }
    for j in crease + 1..h {
        for i in 0..w {
            let v = j * w + i;
            let out = (j - crease) as f32 * SPACING;
            state.pz[v] = hinge_z - out * OPEN[0];
            state.py[v] = hinge_y + out * OPEN[1];
        }
    }
    state
}

/// What a seam sews together is not cloth passing through itself.
///
/// Two panels drawn up a gap apart and sewn along the edges that face each
/// other. The sewing pulls them onto one line, and every triangle round one
/// side then reaches across to the other — which a pass that knew only about
/// triangles would part, fighting the seam for as long as it took to close.
#[test]
fn a_seam_is_not_a_collision_with_itself() {
    let (state, cons, tris, seams) = sewn();
    let mut plain = state.clone();
    let mut aware = state.clone();
    let mut blind = state;
    let n = plain.len();
    let mut knows = Layers::of(&tris, &cons, &seams, n);
    let mut knows_not = Layers::of(&tris, &cons, &Seams::default(), n);
    let sdf = nowhere();
    for _ in 0..STEPS {
        substep(&mut plain, &cons, &seams, &sdf, Floor::none(), None, DT);
        let on = Some(&mut knows);
        substep(&mut aware, &cons, &seams, &sdf, Floor::none(), on, DT);
        let off = Some(&mut knows_not);
        substep(&mut blind, &cons, &seams, &sdf, Floor::none(), off, DT);
    }
    assert_eq!(knows.contacts(), 0, "the seam was left to close in peace");
    assert_eq!(
        position_hash(&plain),
        position_hash(&aware),
        "a garment being sewn drapes exactly as it did before"
    );
    assert!(
        knows_not.contacts() > 0,
        "and the control says the sewing is what spared it, not the distance"
    );
}

/// Two panels a gap apart, sewn along the edges that face each other.
///
/// The seam is soft, the way a product is let go: a firm one closes this gap
/// inside two substeps and the shock crumples both panels, which is a real
/// contact and not the one under test.
fn sewn() -> (State, DistanceConstraints, Vec<u32>, Seams) {
    const W: usize = 6;
    let (left, tris) = sheet(W, W, [0.0, 0.20, 0.0]);
    let (right, _) = sheet(W, W, [(W - 1) as f32 * SPACING + GAP, 0.20, 0.0]);
    let base = (W * W) as u32;
    let mut state = State::new(W * W * 2);
    for v in 0..W * W {
        for (axis, from) in [
            (&mut state.px, (&left.px, &right.px)),
            (&mut state.py, (&left.py, &right.py)),
            (&mut state.pz, (&left.pz, &right.pz)),
        ] {
            axis[v] = from.0[v];
            axis[v + W * W] = from.1[v];
        }
    }
    let both: Vec<u32> = tris
        .iter()
        .copied()
        .chain(tris.iter().map(|&v| v + base))
        .collect();
    let cons = stretch(&state, &both);
    let seams = Seams {
        a: (0..W).map(|j| (j * W + W - 1) as u32).collect(),
        b: (0..W).map(|j| base + (j * W) as u32).collect(),
        compliance: 1.0e-5,
        max_step: 0.002,
        iterations: 4,
    };
    (state, cons, both, seams)
}

/// The thickness is the mesh's own, and a mesh with no edges has none.
#[test]
fn the_thickness_comes_off_the_mesh() {
    let (state, tris) = sheet(8, 8, [0.0, 0.0, 0.0]);
    let cons = stretch(&state, &tris);
    let layers = Layers::of(&tris, &cons, &Seams::default(), state.len());
    assert_eq!(layers.thickness(), mean_rest(&cons) * THICKNESS_OF_EDGE);
    assert!(layers.thickness() > 0.0 && layers.thickness() < SPACING);

    let bare = Layers::of(
        &tris,
        &DistanceConstraints::default(),
        &Seams::default(),
        64,
    );
    assert_eq!(bare.thickness(), 0.0, "nothing to read an edge off");
}
