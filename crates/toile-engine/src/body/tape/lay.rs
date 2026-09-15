use toile_anny::asset::{PathId, RingId};
use toile_anny::measure::{Ring, at, centroid, floor_and_crown, path, ring};

use super::{BodyMesh, Lay, Tape};

/// The body's own right, where the asset bakes the leg and arm paths: the side
/// the stature rule stands on too, so every straight-down tape is seen from
/// the same side.
const RIGHT: [f32; 3] = [-1.0, 0.0, 0.0];

/// Lays one measurement on `mesh`.
pub(super) fn on(how: Lay, mesh: &BodyMesh) -> Tape {
    match how {
        Lay::Girth(id) => girth(mesh, id),
        Lay::Skin(id) => skin(mesh, id),
        Lay::Stature => stature(mesh),
    }
}

/// A ring, measured flat and drawn on the skin.
///
/// Measured: every crossing moved along the plane's normal into the plane
/// through the centroid, which leaves each chord exactly the in-plane chord
/// `measure` sums. Drawn: the crossings themselves, where the ring meets the
/// skin.
///
/// The two cannot be one line. A morph buckles the ring out of its plane, by
/// three centimetres at the upper chest of the reference body and nearly five
/// round a heavy neck, so the flattened loop runs under the skin in one place
/// and centimetres off it in the next, and no push along the skin's normal
/// puts it back: how far each point sank is not known, only guessed. The
/// crossings lie on the mesh's own edges, on the skin on every body. What
/// that costs is length: the drawn loop runs longer than the measurement by
/// the very buckling the measurement flattens out.
fn girth(mesh: &BodyMesh, id: RingId) -> Tape {
    let Ring { points, normal } = ring(id);
    let c = centroid(&mesh.positions, points);
    let drawn: Vec<[f32; 3]> = points.iter().map(|&p| at(&mesh.positions, p)).collect();
    let flat = drawn
        .iter()
        .map(|&skin| sub(skin, scale(normal, dot(sub(skin, c), normal))))
        .collect();
    Tape {
        points: flat,
        closed: true,
        drawn,
        out: points.iter().map(|&p| unit(at(&mesh.normals, p))).collect(),
    }
}

/// A path exactly where `measure` walks it, measured and drawn alike: every
/// point is the skin itself. The inseam goes on plumb to the floor under its
/// last point, and that end is lifted as the last point is, so the drop stays
/// plumb.
fn skin(mesh: &BodyMesh, id: PathId) -> Tape {
    let baked = path(id);
    let mut points: Vec<[f32; 3]> = baked.iter().map(|&p| at(&mesh.positions, p)).collect();
    let mut out: Vec<[f32; 3]> = baked.iter().map(|&p| unit(at(&mesh.normals, p))).collect();
    if id.ends_on_floor()
        && let (Some(&[x, _, z]), Some(&last_out)) = (points.last(), out.last())
    {
        let (floor, _) = floor_and_crown(&mesh.positions);
        points.push([x, floor, z]);
        out.push(last_out);
    }
    Tape {
        drawn: points.clone(),
        points,
        closed: false,
        out,
    }
}

/// A plumb line from the crown's height to the floor, standing beside the
/// body the way the rule a stature is read against stands beside a person:
/// level with the head, and as far to the right as the body reaches, the hand
/// included, so nothing of the body stands between it and the side.
fn stature(mesh: &BodyMesh) -> Tape {
    let (floor, crown) = floor_and_crown(&mesh.positions);
    let side = mesh
        .positions
        .iter()
        .step_by(3)
        .fold(f32::INFINITY, |least, &x| least.min(x));
    let z = centroid(&mesh.positions, ring(RingId::Head).points)[2];
    let points = vec![[side, crown, z], [side, floor, z]];
    Tape {
        drawn: points.clone(),
        points,
        closed: false,
        out: vec![RIGHT; 2],
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// `a` scaled to unit length, or zero when it has none to scale.
fn unit(a: [f32; 3]) -> [f32; 3] {
    let len = dot(a, a).sqrt();
    if len > 0.0 {
        scale(a, 1.0 / len)
    } else {
        [0.0; 3]
    }
}
