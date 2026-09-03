use super::sdf::SdfGrid;

/// Fraction of the substep's tangential motion removed on contact. Without
/// friction the garment slides down the field forever and never settles.
const FRICTION: f32 = 0.5;

/// Contact damping, applied by moving `q` toward `p`. PBD velocity is
/// `(p − q)/dt`, so this bleeds off the energy that normal jitter from the
/// trilinear field pumps in, without touching cloth in free flight.
const CONTACT_DAMP: f32 = 0.5;

/// Projects one particle out of the field and applies contact friction and
/// damping. A particle in free flight is returned untouched.
///
/// Takes and returns both the current position `p` and the pre-substep
/// position `q`, because damping contact means moving `q`, not `p`.
#[inline]
pub(super) fn resolve(sdf: &SdfGrid, eps: f32, p: [f32; 3], q: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let d = sdf.sample(p[0], p[1], p[2]);
    if d >= 0.0 {
        return (p, q);
    }
    let gx = sdf.sample(p[0] + eps, p[1], p[2]) - d;
    let gy = sdf.sample(p[0], p[1] + eps, p[2]) - d;
    let gz = sdf.sample(p[0], p[1], p[2] + eps) - d;
    let glen = (gx * gx + gy * gy + gz * gz).sqrt().max(1.0e-9);
    let push = -d / glen;
    let mut p = [p[0] + gx * push, p[1] + gy * push, p[2] + gz * push];

    let (nx, ny, nz) = (gx / glen, gy / glen, gz / glen);
    let (mx, my, mz) = (p[0] - q[0], p[1] - q[1], p[2] - q[2]);
    let dn = mx * nx + my * ny + mz * nz;
    p[0] -= FRICTION * (mx - dn * nx);
    p[1] -= FRICTION * (my - dn * ny);
    p[2] -= FRICTION * (mz - dn * nz);

    let q = [
        q[0] + CONTACT_DAMP * (p[0] - q[0]),
        q[1] + CONTACT_DAMP * (p[1] - q[1]),
        q[2] + CONTACT_DAMP * (p[2] - q[2]),
    ];
    (p, q)
}
