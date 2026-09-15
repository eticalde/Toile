use toile_anny::phenotype::Phenotype;

use super::super::*;
use super::reference;
use crate::body::body_mesh;
use crate::draft::MeasureSet;

/// How far these tests lift a tape to draw it: the viewport's own gap.
const GAP_M: f32 = 0.003;

/// How much further than the gap a drawn point may stand from the surface: a
/// tenth of a millimetre.
///
/// A drawn point is a point of the skin moved the gap along a unit normal, so
/// the surface cannot be further from it than the gap, and what the margin
/// holds is f32 rounding, micrometres. A tape drawn anywhere else is out by
/// millimetres: a girth flattened into its plane and pushed back out along
/// the skin stands up to two centimetres off a heavy body.
const MARGIN_M: f64 = 0.0001;

/// The reference body, and the heavy corners of the phenotype box, where a
/// morph buckles the rings furthest out of their planes.
fn bodies() -> [(&'static str, BodyMesh); 3] {
    let heavy = |gender, muscle| {
        let phenotype = Phenotype {
            gender,
            age: 0.8,
            muscle,
            weight: 1.0,
            height: 0.5,
            proportions: 0.5,
        };
        body_mesh(&phenotype, &[0.0; 20])
    };
    [
        ("reference", reference()),
        ("heavy male", heavy(0.0, 1.0)),
        ("heavy female", heavy(1.0, 0.0)),
    ]
}

/// Every drawn tape lies a gap off the skin and on its outer side, on every
/// body in [`bodies`].
///
/// Not two of the tape's own points compared, but the drawn ribbon against
/// the triangles the body is rendered as. The stature stands beside the body
/// and the inseam's drop to the floor hangs plumb, both on purpose, so neither
/// is held to it.
#[test]
fn every_drawn_tape_lies_a_gap_off_the_skin() {
    let allowed = 0.0..=f64::from(GAP_M) + MARGIN_M;
    let mut astray = Vec::new();
    for (body, mesh) in bodies() {
        let surface = Surface::of(&mesh);
        for name in MeasureSet::CATALOGUE {
            let in_the_air = match lay(name) {
                Some(Lay::Stature) => continue,
                Some(Lay::Skin(id)) if id.ends_on_floor() => 1,
                _ => 0,
            };
            let drawn = tape(name, &mesh).expect("a catalogue name").lifted(GAP_M);
            let heights = drawn[..drawn.len() - in_the_air]
                .iter()
                .map(|&p| surface.height(p));
            let (lowest, highest) = heights
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), h| {
                    (lo.min(h), hi.max(h))
                });
            if !allowed.contains(&lowest) || !allowed.contains(&highest) {
                astray.push(format!(
                    "{body} {name}: {:.2} to {:.2} mm",
                    lowest * 1000.0,
                    highest * 1000.0
                ));
            }
        }
    }
    assert!(
        astray.is_empty(),
        "drawn points off the skin by more than {GAP_M} m, or under it:\n{}",
        astray.join("\n")
    );
}

type V = [f64; 3];

/// The body's triangles, each with its bounding box, so a query can pass over
/// every triangle that cannot hold the nearest point without solving for it.
struct Surface {
    triangles: Vec<[V; 3]>,
    boxes: Vec<(V, V)>,
}

impl Surface {
    fn of(mesh: &BodyMesh) -> Self {
        let corner = |i: u32| {
            let i = i as usize * 3;
            [0, 1, 2].map(|k| f64::from(mesh.positions[i + k]))
        };
        let triangles: Vec<[V; 3]> = mesh
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| [corner(t[0]), corner(t[1]), corner(t[2])])
            .collect();
        let boxes = triangles
            .iter()
            .map(|t| {
                let lo = [0, 1, 2].map(|k| t[0][k].min(t[1][k]).min(t[2][k]));
                let hi = [0, 1, 2].map(|k| t[0][k].max(t[1][k]).max(t[2][k]));
                (lo, hi)
            })
            .collect();
        Self { triangles, boxes }
    }

    /// How far `p` stands from the nearest point of the surface, negative
    /// when it stands under the skin: behind the nearest triangle's face,
    /// whose counter-clockwise winding faces out of the body.
    fn height(&self, p: [f32; 3]) -> f64 {
        let p = p.map(f64::from);
        let mut best = f64::INFINITY;
        let mut side = 1.0;
        for (t, (lo, hi)) in self.triangles.iter().zip(&self.boxes) {
            let outside = [0, 1, 2].map(|k| (lo[k] - p[k]).max(p[k] - hi[k]).max(0.0));
            if dot(outside, outside) > best {
                continue;
            }
            let off = sub(p, nearest_on_triangle(p, t));
            let d = dot(off, off);
            if d < best {
                best = d;
                side = dot(off, cross(sub(t[1], t[0]), sub(t[2], t[0]))).signum();
            }
        }
        side * best.sqrt()
    }
}

/// The point of triangle `t` nearest `p`: the corner, edge or face region `p`
/// projects into, found from the signs of its barycentric coordinates.
fn nearest_on_triangle(p: V, t: &[V; 3]) -> V {
    let [a, b, c] = *t;
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = sub(p, b);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let cp = sub(p, c);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return along(a, ab, d1 / (d1 - d3));
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return along(a, ac, d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        return along(b, sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let sum = va + vb + vc;
    along(along(a, ab, vb / sum), ac, vc / sum)
}

fn along(a: V, d: V, k: f64) -> V {
    [0, 1, 2].map(|i| a[i] + k * d[i])
}

fn sub(a: V, b: V) -> V {
    [0, 1, 2].map(|i| a[i] - b[i])
}

fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
