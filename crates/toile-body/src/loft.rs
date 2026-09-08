use crate::params::{BodyMeasures, BodyRes};
use crate::ring::{Ring, axes_for_girth, point};

// Region shapes: ρ = depth/width and the exponent n in {2, 4}. Torso and hip
// use n = 4 (a rounded square: flatter front and back, a real waist); limbs and
// the dome use n = 2 (ellipse and circle).
const WAIST_RHO: f64 = 0.72;
const WAIST_N: f64 = 4.0;
const HIP_RHO: f64 = 0.78;
const HIP_N: f64 = 4.0;
const LIMB_RHO: f64 = 0.90;
const LIMB_N: f64 = 2.0;
const DOME_RHO: f64 = 0.85;
const DOME_N: f64 = 2.0;
// The ankle joint sits this far above the floor; the crown is `height` above
// it.
const ANKLE_FLOOR: f64 = 0.07;

/// The vertical landmarks, in metres, y up from the ankle joint at 0. Ordered
/// `0 < knee < crotch < hip < waist < crown` by construction so the loft never
/// folds back on itself, whatever the measurements say.
pub(crate) struct Landmarks {
    pub knee: f64,
    pub crotch: f64,
    pub hip: f64,
    pub waist: f64,
    pub crown: f64,
}

/// Places the landmarks from a measure set, clamping so their order always
/// holds (ISO 8559 semantics: outseam is waist→ankle, rise is waist→crotch,
/// `hip_drop` is waist→hip; inseam is redundant with outseam and rise and is
/// not used as a second crotch anchor).
pub(crate) fn landmarks(m: &BodyMeasures) -> Landmarks {
    let cm = 0.01;
    let crown = (m.height * cm - ANKLE_FLOOR).max(0.5);
    let waist = (m.outseam * cm).clamp(0.10, crown - 0.05);
    let crotch = (waist - m.rise * cm).clamp(waist * 0.1, waist - 0.02);
    let mut hip = waist - (m.hip_drop * cm).max(0.005);
    if hip <= crotch || hip >= waist {
        hip = f64::midpoint(crotch, waist);
    }
    Landmarks {
        knee: crotch * 0.5,
        crotch,
        hip,
        waist,
        crown,
    }
}

/// One cross-section as the loft interpolates it, in metres.
struct Sec {
    a: f64,
    b: f64,
    n: f64,
    cx: f64,
    y: f64,
}

fn lerp(p: f64, q: f64, t: f64) -> f64 {
    p + (q - p) * t
}

fn interp(x: &Sec, z: &Sec, t: f64) -> Sec {
    Sec {
        a: lerp(x.a, z.a, t),
        b: lerp(x.b, z.b, t),
        n: lerp(x.n, z.n, t),
        cx: lerp(x.cx, z.cx, t),
        y: lerp(x.y, z.y, t),
    }
}

fn ring_of(s: &Sec, dirs: &[(f64, f64)]) -> Vec<[f32; 3]> {
    let r = Ring {
        cx: s.cx,
        cz: 0.0,
        y: s.y,
        a: s.a,
        b: s.b,
        n: s.n,
    };
    dirs.iter().map(|&d| point(&r, d)).collect()
}

/// The rings of a tube through ordered sections, with `steps` interpolated
/// rings inside each span and the shared landmarks emitted once.
fn tube(secs: &[Sec], steps: u32, dirs: &[(f64, f64)]) -> Vec<Vec<[f32; 3]>> {
    let mut rings = Vec::new();
    let denom = f64::from(steps + 1);
    for w in secs.windows(2) {
        for k in 0..=steps {
            rings.push(ring_of(&interp(&w[0], &w[1], f64::from(k) / denom), dirs));
        }
    }
    if let Some(last) = secs.last() {
        rings.push(ring_of(last, dirs));
    }
    rings
}

/// The trunk and the dome above it: crotch → hip → waist, then a sqrt-profiled
/// dome that closes on a single crown vertex. The bottom is capped (hidden
/// between the legs); the dome apex closes itself.
pub(crate) fn trunk(
    m: &BodyMeasures,
    res: BodyRes,
    lm: &Landmarks,
    dirs: &[(f64, f64)],
) -> (Vec<f32>, Vec<u32>) {
    let (a_hip, b_hip) = axes_for_girth(m.hip, HIP_RHO, HIP_N, dirs);
    // The crotch section blends the hip and thigh girths as the trunk narrows
    // toward the legs.
    let crotch_girth = f64::midpoint(m.hip, m.thigh);
    let secs = [
        sec(crotch_girth, HIP_RHO, HIP_N, 0.0, lm.crotch, dirs),
        Sec {
            a: a_hip,
            b: b_hip,
            n: HIP_N,
            cx: 0.0,
            y: lm.hip,
        },
        sec(m.waist, WAIST_RHO, WAIST_N, 0.0, lm.waist, dirs),
    ];
    let mut rings = tube(&secs, res.loft_steps, dirs);

    let (a_waist, _) = axes_for_girth(m.waist, WAIST_RHO, WAIST_N, dirs);
    let base = 1.15 * a_waist; // shoulders bulge past the waist by a fixed ratio
    let steps = f64::from(res.dome_rings);
    for i in 0..=res.dome_rings {
        let t = f64::from(i) / steps;
        let a = base * (1.0 - t).sqrt(); // 1 at the waist, 0 at the crown
        rings.push(ring_of(
            &Sec {
                a,
                b: a * DOME_RHO,
                n: DOME_N,
                cx: 0.0,
                y: lerp(lm.waist, lm.crown, t),
            },
            dirs,
        ));
    }
    loft(&rings, true, false)
}

/// One leg, centred at `cx`: ankle → knee → thigh, capped at both ends. The
/// thigh top overlaps the trunk at the crotch (hidden interpenetration, which a
/// dummy accepts; a watertight pelvis merge is a follow-up).
pub(crate) fn leg(
    m: &BodyMeasures,
    res: BodyRes,
    lm: &Landmarks,
    cx: f64,
    dirs: &[(f64, f64)],
) -> (Vec<f32>, Vec<u32>) {
    let secs = [
        sec(m.ankle, LIMB_RHO, LIMB_N, cx, 0.0, dirs),
        sec(m.knee, LIMB_RHO, LIMB_N, cx, lm.knee, dirs),
        sec(m.thigh, LIMB_RHO, LIMB_N, cx, lm.crotch, dirs),
    ];
    loft(&tube(&secs, res.loft_steps, dirs), true, true)
}

/// The leg spacing: half the hip half-width, so the legs stand apart under the
/// pelvis (pose v1, legs apart at the hip).
pub(crate) fn hip_half_width(m: &BodyMeasures, dirs: &[(f64, f64)]) -> f64 {
    axes_for_girth(m.hip, HIP_RHO, HIP_N, dirs).0 * 0.5
}

fn sec(girth: f64, rho: f64, n: f64, cx: f64, y: f64, dirs: &[(f64, f64)]) -> Sec {
    let (a, b) = axes_for_girth(girth, rho, n, dirs);
    Sec { a, b, n, cx, y }
}

/// Grid-triangulates consecutive rings (each `seg + 1` wide, seam duplicated)
/// into CCW-outward quads, optionally fan-capping the first and last ring.
pub(crate) fn loft(rings: &[Vec<[f32; 3]>], cap_lo: bool, cap_hi: bool) -> (Vec<f32>, Vec<u32>) {
    let width = rings.first().map_or(0, Vec::len);
    let seg = width.saturating_sub(1) as u32;
    let mut verts: Vec<f32> = Vec::new();
    for ring in rings {
        for p in ring {
            verts.extend_from_slice(p);
        }
    }
    let mut idx: Vec<u32> = Vec::new();
    let width_u = width as u32;
    for r in 0..rings.len().saturating_sub(1) as u32 {
        let (lo, hi) = (r * width_u, (r + 1) * width_u);
        for j in 0..seg {
            let (a, b) = (lo + j, hi + j);
            // Rings run bottom→top, columns +x→+z: this winding faces outward.
            idx.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    if cap_lo && !rings.is_empty() {
        fan(&mut verts, &mut idx, &rings[0], 0, true);
    }
    if cap_hi && rings.len() > 1 {
        let start = (rings.len() - 1) as u32 * width_u;
        fan(&mut verts, &mut idx, rings.last().unwrap(), start, false);
    }
    (verts, idx)
}

/// Fans a ring to a fresh centroid vertex. `lo` caps face −y, `hi` caps face
/// +y.
fn fan(verts: &mut Vec<f32>, idx: &mut Vec<u32>, ring: &[[f32; 3]], start: u32, lo: bool) {
    let seg = ring.len() - 1;
    let (mut cx, mut cy, mut cz) = (0.0f32, 0.0f32, 0.0f32);
    for p in &ring[..seg] {
        cx += p[0];
        cy += p[1];
        cz += p[2];
    }
    let inv = 1.0 / seg as f32;
    let center = (verts.len() / 3) as u32;
    verts.extend_from_slice(&[cx * inv, cy * inv, cz * inv]);
    for j in 0..seg as u32 {
        let (a, b) = (start + j, start + j + 1);
        if lo {
            idx.extend_from_slice(&[center, b, a]);
        } else {
            idx.extend_from_slice(&[center, a, b]);
        }
    }
}

/// Area-weighted vertex normals: accumulate each triangle's `cross(p1-p0,
/// p2-p0)` into its three vertices, then normalize. Consistent CCW winding
/// makes them point outward; a degenerate vertex with no face falls back to up.
pub(crate) fn vertex_normals(positions: &[f32], tris: &[u32]) -> Vec<f32> {
    let mut out = vec![0.0f32; positions.len()];
    let at = |i: u32| {
        let i = i as usize * 3;
        [positions[i], positions[i + 1], positions[i + 2]]
    };
    for t in tris.as_chunks::<3>().0 {
        let (pa, pb, pc) = (at(t[0]), at(t[1]), at(t[2]));
        let e1 = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
        let e2 = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        for &v in t {
            let v = v as usize * 3;
            out[v] += n[0];
            out[v + 1] += n[1];
            out[v + 2] += n[2];
        }
    }
    for nrm in out.as_chunks_mut::<3>().0 {
        let len = (nrm[0] * nrm[0] + nrm[1] * nrm[1] + nrm[2] * nrm[2]).sqrt();
        if len < 1.0e-12 {
            // A collapsed apex or cap-centre vertex has no face normal; give it
            // the up axis so every normal the renderer reads is unit length.
            nrm[1] = 1.0;
        } else {
            nrm[0] /= len;
            nrm[1] /= len;
            nrm[2] /= len;
        }
    }
    out
}
