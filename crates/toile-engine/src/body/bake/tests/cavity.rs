use std::collections::HashMap;

use super::{BAND, CELL, bits};
use crate::body::bake::{Lattice, inspect, mesh_sdf};

/// The solid block's far corner, in metres. Its near corner is the origin.
const OUTER: f32 = 0.26;

/// The chamber's near and far corners on every axis: a cube of air with no
/// way out but the neck.
const CHAMBER: [f32; 2] = [0.08, 0.18];

/// The neck's near and far corners across x and z. It runs from the chamber's
/// ceiling to the top of the block.
///
/// Four centimetres across, against a band of two and a half: every sample in
/// the neck is within the band of one of its walls, so the neck is a wall the
/// flood cannot pass. That is the whole point of the fixture — the chamber is
/// open to the air and unreachable from the rim at the same time.
const NECK: [f32; 2] = [0.11, 0.15];

/// A mesh assembled face by face, each turned to face the way it is told
/// rather than by hand-winding its corners.
#[derive(Default)]
struct Block {
    positions: Vec<f32>,
    indices: Vec<u32>,
    seen: HashMap<[u32; 3], u32>,
}

impl Block {
    /// The index of one corner, shared with every face that names it.
    fn vertex(&mut self, p: [f32; 3]) -> u32 {
        let key = p.map(f32::to_bits);
        if let Some(&i) = self.seen.get(&key) {
            return i;
        }
        let i = (self.positions.len() / 3) as u32;
        self.positions.extend_from_slice(&p);
        self.seen.insert(key, i);
        i
    }

    /// One planar convex polygon, wound so that its normal points `out`.
    fn face(&mut self, poly: &[[f32; 3]], out: [f32; 3]) {
        let p = |k: usize| poly[k].map(f64::from);
        let e1 = [0, 1, 2].map(|c| p(1)[c] - p(0)[c]);
        let e2 = [0, 1, 2].map(|c| p(2)[c] - p(0)[c]);
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let forward: f64 = (0..3).map(|c| n[c] * f64::from(out[c])).sum();
        let v: Vec<u32> = poly.iter().map(|c| self.vertex(*c)).collect();
        for k in 1..v.len() - 1 {
            if forward > 0.0 {
                self.indices.extend_from_slice(&[v[0], v[k], v[k + 1]]);
            } else {
                self.indices.extend_from_slice(&[v[0], v[k + 1], v[k]]);
            }
        }
    }

    /// A rectangular ring in one horizontal plane, mitred at the corners so
    /// that no corner of the hole lands in the middle of another quad's edge
    /// and tears the surface into something non-manifold.
    fn ring(&mut self, y: f32, o: [f32; 4], i: [f32; 4], out: [f32; 3]) {
        let c = |x: f32, z: f32| [x, y, z];
        for strip in [
            [c(o[0], o[2]), c(o[1], o[2]), c(i[1], i[2]), c(i[0], i[2])],
            [c(o[1], o[2]), c(o[1], o[3]), c(i[1], i[3]), c(i[1], i[2])],
            [c(o[1], o[3]), c(o[0], o[3]), c(i[0], i[3]), c(i[1], i[3])],
            [c(o[0], o[3]), c(o[0], o[2]), c(i[0], i[2]), c(i[0], i[3])],
        ] {
            self.face(&strip, out);
        }
    }

    /// The four walls of an axis-aligned shaft.
    ///
    /// `facing` is what tells a hollow from a block: `1.0` when the solid is
    /// outside the shaft and the walls face into it, `-1.0` when the solid is
    /// the shaft and they face out of it.
    fn shaft(&mut self, r: [f32; 2], lo: f32, hi: f32, facing: f32) {
        for axis in [0usize, 2] {
            for (end, sign) in [(r[0], 1.0f32), (r[1], -1.0)] {
                let mut a = [0.0; 3];
                let mut b = [0.0; 3];
                a[axis] = end;
                b[axis] = end;
                let other = 2 - axis;
                a[other] = r[0];
                b[other] = r[1];
                let (mut p0, mut p1) = (a, b);
                p0[1] = lo;
                p1[1] = lo;
                let (mut p2, mut p3) = (b, a);
                p2[1] = hi;
                p3[1] = hi;
                let mut out = [0.0; 3];
                out[axis] = sign * facing;
                self.face(&[p0, p1, p2, p3], out);
            }
        }
    }
}

/// A solid block with a narrow-mouthed chamber hollowed out of its top.
fn block() -> Block {
    let mut b = Block::default();
    let (o, c, n) = (OUTER, CHAMBER, NECK);
    // The five whole outer faces.
    b.face(
        &[[0.0; 3], [o, 0.0, 0.0], [o, 0.0, o], [0.0, 0.0, o]],
        [0.0, -1.0, 0.0],
    );
    b.shaft([0.0, o], 0.0, o, -1.0);
    // The top, with the neck's mouth in it, and the neck below it.
    b.ring(
        o,
        [0.0, o, 0.0, o],
        [n[0], n[1], n[0], n[1]],
        [0.0, 1.0, 0.0],
    );
    b.shaft(n, c[1], o, 1.0);
    // The chamber: a ceiling with the neck's foot in it, four walls, a floor.
    b.ring(
        c[1],
        [c[0], c[1], c[0], c[1]],
        [n[0], n[1], n[0], n[1]],
        [0.0, -1.0, 0.0],
    );
    b.shaft(c, c[0], c[1], 1.0);
    b.face(
        &[
            [c[0], c[0], c[0]],
            [c[1], c[0], c[0]],
            [c[1], c[0], c[1]],
            [c[0], c[0], c[1]],
        ],
        [0.0, 1.0, 0.0],
    );
    b
}

/// Whether a point is in the solid, by the arithmetic that placed the corners
/// and by nothing the bake ever touches.
fn solid(q: [f64; 3]) -> bool {
    let within = |lo: f32, hi: f32, v: f64| v > f64::from(lo) && v < f64::from(hi);
    let block = (0..3).all(|c| within(0.0, OUTER, q[c]));
    let neck = within(NECK[0], NECK[1], q[0])
        && within(NECK[0], NECK[1], q[2])
        && within(CHAMBER[1], OUTER, q[1]);
    block && !in_chamber(q) && !neck
}

/// Whether a point is in the chamber.
fn in_chamber(q: [f64; 3]) -> bool {
    (0..3).all(|c| q[c] > f64::from(CHAMBER[0]) && q[c] < f64::from(CHAMBER[1]))
}

/// The shape is the one the fixture claims: closed, orientable, one surface,
/// and hollow by exactly the chamber and the neck.
#[test]
fn the_hollowed_block_is_one_closed_surface() {
    let b = block();
    let m = inspect(&b.positions, &b.indices);
    assert_eq!(m.boundary_edges, 0, "no hole");
    assert_eq!(m.nonmanifold_edges, 0, "no edge with three faces on it");
    assert_eq!(m.inconsistent_edges, 0, "no flipped face");
    assert_eq!(m.degenerate_triangles, 0, "every face has a normal");
    assert_eq!(m.euler, 2, "a dimple leaves it a sphere");
    let litres = f64::from(OUTER).powi(3) * 1000.0
        - f64::from(CHAMBER[1] - CHAMBER[0]).powi(3) * 1000.0
        - f64::from(NECK[1] - NECK[0]).powi(2) * f64::from(OUTER - CHAMBER[1]) * 1000.0;
    assert!((m.litres - litres).abs() < 1.0e-6, "{} litres", m.litres);
    assert!(m.earns_the_sign());
}

/// Air the band seals off from the rim is still air.
///
/// The chamber is wider than the band on every axis, so its core is a region
/// the band never writes; its neck is narrower, so the band walls that core
/// off from the box's rim. A flood that reads "could not be reached" as
/// "enclosed by the skin" fills the chamber with flesh, which is what this
/// catches — and it catches it on forty-four triangles, in a debug build, in
/// the ordinary test pass.
#[test]
fn a_chamber_the_band_seals_off_reads_as_air() {
    let b = block();
    let field = mesh_sdf(&b.positions, &b.indices, CELL, BAND).expect("the block is closed");
    let lattice = Lattice::around(&b.positions, CELL, BAND);
    let band = BAND as f32;

    let (mut wrong, mut sealed, mut buried) = (0, 0, 0);
    for k in 0..lattice.dims[2] {
        for j in 0..lattice.dims[1] {
            for i in 0..lattice.dims[0] {
                let q = lattice.point(i, j, k);
                let got = field.data[lattice.index(i, j, k)];
                if (got < 0.0) != solid(q) {
                    wrong += 1;
                }
                if bits(got) == bits(band) && in_chamber(q) {
                    sealed += 1;
                }
                if bits(got) == bits(-band) {
                    buried += 1;
                }
            }
        }
    }
    assert_eq!(wrong, 0, "samples on the wrong side of the skin");
    assert!(
        sealed > 20,
        "only {sealed} saturated samples in the chamber"
    );
    assert!(
        buried > 0,
        "no sample is deep enough in the block to saturate"
    );
}
